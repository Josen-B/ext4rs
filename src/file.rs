use alloc::vec::Vec;
use axdriver_block::BlockDriverOps;
use log::*;

use crate::{Ext4Error, Ext4Result, Inode};

/// File operations
pub struct File {
    inode: Inode,
    position: u64,
}

impl File {
    /// Create a new file from an inode
    pub fn new(inode: Inode) -> Self {
        Self { inode, position: 0 }
    }

    /// Get the inode
    pub fn inode(&self) -> &Inode {
        &self.inode
    }

    /// Get the file size
    pub fn size(&self) -> u64 {
        self.inode.size
    }

    /// Get the current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Seek to a position
    pub fn seek(&mut self, offset: u64) -> Ext4Result<u64> {
        if offset > self.inode.size {
            return Err(Ext4Error::InvalidInput);
        }

        self.position = offset;
        Ok(offset)
    }

    /// Seek from current position
    pub fn seek_from_current(&mut self, offset: i64) -> Ext4Result<u64> {
        let new_pos = if offset >= 0 {
            self.position.checked_add(offset as u64)
        } else {
            self.position.checked_sub((-offset) as u64)
        };

        match new_pos {
            Some(pos) if pos <= self.inode.size => {
                self.position = pos;
                Ok(pos)
            }
            _ => Err(Ext4Error::InvalidInput),
        }
    }

    /// Seek from end
    pub fn seek_from_end(&mut self, offset: i64) -> Ext4Result<u64> {
        let new_pos = if offset >= 0 {
            self.inode.size.checked_add(offset as u64)
        } else {
            self.inode.size.checked_sub((-offset) as u64)
        };

        match new_pos {
            Some(pos) => {
                self.position = pos;
                Ok(pos)
            }
            _ => Err(Ext4Error::InvalidInput),
        }
    }

    /// Read data from the file
    pub fn read<D>(
        &mut self,
        buf: &mut [u8],
        fs: &mut crate::Ext4FileSystem<D>,
    ) -> Ext4Result<usize>
    where
        D: axdriver_block::BlockDriverOps,
    {
        if self.position >= self.inode.size {
            return Ok(0);
        }

        let block_size = fs.superblock().block_size();
        let mut bytes_read = 0;
        let mut offset = self.position;

        while bytes_read < buf.len() && offset < self.inode.size {
            let block_num = self.inode.get_block_number(offset, block_size, fs)?;
            if block_num == 0 {
                // Sparse file - zero block
                let block_offset = (offset % block_size as u64) as usize;
                let remaining_in_block =
                    (block_size as usize - block_offset).min(buf.len() - bytes_read);

                for i in 0..remaining_in_block {
                    buf[bytes_read + i] = 0;
                }

                bytes_read += remaining_in_block;
                offset += remaining_in_block as u64;
                continue;
            }

            // Check if block number is valid
            if block_num >= fs.superblock().blocks_count() as u32 {
                warn!("Invalid block number {} in file inode {}, treating as zero", block_num, self.inode.ino);
                // Treat as sparse block
                let block_offset = (offset % block_size as u64) as usize;
                let remaining_in_block =
                    (block_size as usize - block_offset).min(buf.len() - bytes_read);

                for i in 0..remaining_in_block {
                    buf[bytes_read + i] = 0;
                }

                bytes_read += remaining_in_block;
                offset += remaining_in_block as u64;
                continue;
            }

            let block_offset = (offset % block_size as u64) as usize;
            let remaining_in_block =
                (block_size as usize - block_offset).min(buf.len() - bytes_read);

            let mut block_buf = vec![0u8; block_size as usize];
            if let Err(e) = fs.read_block(block_num, &mut block_buf) {
                warn!("Failed to read block {} for file inode {}: {:?}", block_num, self.inode.ino, e);
                // Treat as sparse block
                for i in 0..remaining_in_block {
                    buf[bytes_read + i] = 0;
                }

                bytes_read += remaining_in_block;
                offset += remaining_in_block as u64;
                continue;
            }

            buf[bytes_read..bytes_read + remaining_in_block]
                .copy_from_slice(&block_buf[block_offset..block_offset + remaining_in_block]);

            bytes_read += remaining_in_block;
            offset += remaining_in_block as u64;
        }

        self.position = offset;
        Ok(bytes_read)
    }

    /// Write data to the file
    pub fn write<D>(&mut self, buf: &[u8], fs: &mut crate::Ext4FileSystem<D>) -> Ext4Result<usize>
    where
        D: BlockDriverOps,
    {
        let block_size = fs.superblock().block_size();
        let mut bytes_written = 0;
        let mut offset = self.position;
        let mut inode = self.inode.clone();

        while bytes_written < buf.len() {
            let block_index = offset / block_size as u64;
            let block_num = match inode.get_block_number(offset, block_size, fs) {
                Ok(0) => {
                    // Need to allocate a new block
                    let new_block = fs.alloc_block()?;
                    inode.set_block(block_index, new_block, block_size, fs)?;
                    new_block
                }
                Ok(block) => {
                    if block >= fs.superblock().blocks_count() as u32 {
                        warn!("Invalid block number {} in file inode {}, allocating new block", block, inode.ino);
                        // Allocate a new block
                        let new_block = fs.alloc_block()?;
                        inode.set_block(block_index, new_block, block_size, fs)?;
                        new_block
                    } else {
                        block
                    }
                }
                Err(_) => {
                    // Need to allocate a new block
                    let new_block = fs.alloc_block()?;
                    inode.set_block(block_index, new_block, block_size, fs)?;
                    new_block
                }
            };

            let block_offset = (offset % block_size as u64) as usize;
            let remaining_in_block =
                (block_size as usize - block_offset).min(buf.len() - bytes_written);

            let mut block_buf = vec![0u8; block_size as usize];

            // Read existing block if not writing to a new block
            if block_offset > 0 || remaining_in_block < block_size as usize {
                if let Err(e) = fs.read_block(block_num, &mut block_buf) {
                    warn!("Failed to read block {} for file inode {}: {:?}", block_num, inode.ino, e);
                    // Continue with zero-filled block
                }
            }

            block_buf[block_offset..block_offset + remaining_in_block]
                .copy_from_slice(&buf[bytes_written..bytes_written + remaining_in_block]);

            fs.write_block(block_num, &block_buf)?;

            bytes_written += remaining_in_block;
            offset += remaining_in_block as u64;
        }

        self.position = offset;

        // Update file size if needed
        if offset > inode.size {
            inode.size = offset;
            // Update block count
            inode.blocks = (offset + block_size as u64 - 1) / block_size as u64;
        }

        // Write updated inode
        fs.write_inode(&inode)?;
        self.inode = inode;

        Ok(bytes_written)
    }

    /// Truncate the file
    pub fn truncate<D>(
        &mut self,
        new_size: u64,
        fs: &mut crate::Ext4FileSystem<D>,
    ) -> Ext4Result<()>
    where
        D: BlockDriverOps,
    {
        let block_size = fs.superblock().block_size();
        let old_block_count = (self.inode.size + block_size as u64 - 1) / block_size as u64;
        let new_block_count = (new_size + block_size as u64 - 1) / block_size as u64;

        if new_size > self.inode.size {
            // Expand file - allocate blocks as needed
            for block_index in old_block_count..new_block_count {
                let new_block = fs.alloc_block()?;
                self.inode
                    .set_block(block_index, new_block, block_size, fs)?;

                // Initialize the new block with zeros
                let zero_buf = vec![0u8; block_size as usize];
                fs.write_block(new_block, &zero_buf)?;
            }
        } else if new_size < self.inode.size {
            // Shrink file - free blocks that are no longer needed
            for block_index in new_block_count..old_block_count {
                if let Ok(block_num) =
                    self.inode
                        .get_block_number(block_index * block_size as u64, block_size, fs)
                {
                    if block_num != 0 {
                        // Free the block
                        // Note: In a complete implementation, we would need to update the block bitmap
                        // For now, we just set the block pointer to 0
                        self.inode.set_block(block_index, 0, block_size, fs)?;
                    }
                }
            }
        }

        // Update the inode size
        self.inode.size = new_size;

        // Adjust position if it's beyond the new file size
        if self.position > new_size {
            self.position = new_size;
        }

        Ok(())
    }
}
