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
            let block_num = self.inode.get_block_number(offset, block_size)?;
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

            let block_offset = (offset % block_size as u64) as usize;
            let remaining_in_block =
                (block_size as usize - block_offset).min(buf.len() - bytes_read);

            let mut block_buf = vec![0u8; block_size as usize];
            fs.read_block(block_num, &mut block_buf)?;

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

        while bytes_written < buf.len() {
            let block_num = self.inode.get_block_number(offset, block_size)?;
            if block_num == 0 {
                // Need to allocate a new block
                let new_block = fs.alloc_block()?;
                // TODO: Update inode block pointer
                // This is a simplified implementation
                return Err(Ext4Error::NotSupported);
            }

            let block_offset = (offset % block_size as u64) as usize;
            let remaining_in_block =
                (block_size as usize - block_offset).min(buf.len() - bytes_written);

            let mut block_buf = vec![0u8; block_size as usize];
            fs.read_block(block_num, &mut block_buf)?;

            block_buf[block_offset..block_offset + remaining_in_block]
                .copy_from_slice(&buf[bytes_written..bytes_written + remaining_in_block]);

            fs.write_block(block_num, &block_buf)?;

            bytes_written += remaining_in_block;
            offset += remaining_in_block as u64;
        }

        self.position = offset;

        // Update file size if needed
        if offset > self.inode.size {
            // TODO: Update inode size
            // This is a simplified implementation
        }

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
        if new_size > self.inode.size {
            // Expand file
            // TODO: Allocate blocks as needed
            return Err(Ext4Error::NotSupported);
        } else if new_size < self.inode.size {
            // Shrink file
            // TODO: Free blocks that are no longer needed
            return Err(Ext4Error::NotSupported);
        }

        Ok(())
    }
}
