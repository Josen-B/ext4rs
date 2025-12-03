use alloc::vec::Vec;
use bitflags::bitflags;
use core::time::Duration;
use log::*;

use crate::{Ext4Error, Ext4Result};

/// Inode types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Directory,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    SymLink,
}

/// Inode mode flags
bitflags! {
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct InodeMode: u16 {
        const IFMT = 0xF000;
        const IFIFO = 0x1000;
        const IFCHR = 0x2000;
        const IFDIR = 0x4000;
        const IFBLK = 0x6000;
        const IFREG = 0x8000;
        const IFLNK = 0xA000;
        const IFSOCK = 0xC000;

        const ISUID = 0x0800;
        const ISGID = 0x0400;
        const ISVTX = 0x0200;

        const IRUSR = 0x0100;
        const IWUSR = 0x0080;
        const IXUSR = 0x0040;
        const IRGRP = 0x0020;
        const IWGRP = 0x0010;
        const IXGRP = 0x0008;
        const IROTH = 0x0004;
        const IWOTH = 0x0002;
        const IXOTH = 0x0001;
    }
}

impl InodeMode {
    pub const DEFAULT_FILE: Self = Self::IFREG
        .union(Self::IRUSR)
        .union(Self::IWUSR)
        .union(Self::IRGRP)
        .union(Self::IROTH);
    pub const DEFAULT_DIR: Self = Self::IFDIR
        .union(Self::IRUSR)
        .union(Self::IWUSR)
        .union(Self::IXUSR)
        .union(Self::IRGRP)
        .union(Self::IXGRP)
        .union(Self::IROTH)
        .union(Self::IXOTH);
}

/// Ext4 inode structure
#[derive(Clone)]
pub struct Inode {
    /// Inode number
    pub ino: u32,
    /// File mode
    pub mode: InodeMode,
    /// User ID
    pub uid: u16,
    /// File size
    pub size: u64,
    /// Access time
    pub atime: u32,
    /// Creation time
    pub ctime: u32,
    /// Modification time
    pub mtime: u32,
    /// Deletion time
    pub dtime: u32,
    /// Group ID
    pub gid: u16,
    /// Links count
    pub links_count: u16,
    /// Blocks count
    pub blocks: u64,
    /// File flags
    pub flags: u32,
    /// Version (used for NFS)
    pub version: u32,
    /// File ACL
    pub file_acl: u32,
    /// Directory ACL or upper 16 bits of file size for large files
    pub dir_acl: u32,
    /// Fragment address
    pub faddr: u32,
    /// Direct block pointers
    pub block: [u32; 15],
    /// Generation number
    pub generation: u32,
    /// Extended attribute block
    pub faddr_ext: u32,
    /// File ACL (high 32 bits)
    pub file_acl_high: u32,
    /// Upper 32 bits of size if needed
    pub size_high: u32,
    /// Obsoleted fragment address
    pub obso_faddr: u32,
    /// Extra inode size
    pub extra_isize: u16,
    /// Checksum
    pub checksum: u16,
    /// Extra timestamps
    pub ctime_extra: u32,
    pub mtime_extra: u32,
    pub atime_extra: u32,
    /// Crtime (creation time)
    pub crtime: u32,
    /// Crtime extra
    pub crtime_extra: u32,
    /// Project ID
    pub projid: u32,
}

impl Inode {
    /// Parse inode from bytes
    pub fn from_bytes(data: &[u8], ino: u32) -> Ext4Result<Self> {
        if data.len() < 128 {
            return Err(Ext4Error::InvalidInput);
        }

        debug!("Parsing inode {} from {} bytes of data", ino, data.len());
        if data.len() >= 16 {
            debug!("First 16 bytes: {:x?}", &data[..16]);
        }

        // Helper function to read little-endian values
        let read_u32 = |offset: usize| -> u32 {
            (data[offset] as u32)
                | ((data[offset + 1] as u32) << 8)
                | ((data[offset + 2] as u32) << 16)
                | ((data[offset + 3] as u32) << 24)
        };

        let read_u16 =
            |offset: usize| -> u16 { (data[offset] as u16) | ((data[offset + 1] as u16) << 8) };

        let _read_u8 = |offset: usize| -> u8 { data[offset] };

        let mode = read_u16(0);
        let uid = read_u16(2);
        let size_lo = read_u32(4);
        let atime = read_u32(8);
        let ctime = read_u32(12);
        let mtime = read_u32(16);
        let dtime = read_u32(20);
        let gid = read_u16(24);
        let links_count = read_u16(26);
        let blocks_lo = read_u32(28);
        let flags = read_u32(32);
        let version = read_u32(36);

        let mut block = [0u32; 15];
        for i in 0..15 {
            block[i] = read_u32(40 + i * 4);
        }

        // Debug: Print first few block pointers with raw bytes
        if ino == 2 {
            debug!("Inode 2 raw bytes at offsets 40-60: {:x?}", &data[40..60]);
            for i in 0..4 {
                let offset = 40 + i * 4;
                let raw_bytes = &data[offset..offset + 4];
                let parsed = read_u32(offset);
                debug!(
                    "Block[{}]: offset={}, bytes={:x?}, parsed={}",
                    i, offset, raw_bytes, parsed
                );
            }
            debug!(
                "Inode 2 block pointers: [{}, {}, {}, {}]",
                block[0], block[1], block[2], block[3]
            );
        }

        let generation = read_u32(100);
        let file_acl = read_u32(104);
        let dir_acl = read_u32(108);
        let faddr = read_u32(112);

        // Skip to extended fields if needed
        let mut extra_isize = 0;
        let mut checksum = 0;
        let mut ctime_extra = 0;
        let mut mtime_extra = 0;
        let mut atime_extra = 0;
        let mut crtime = 0;
        let mut crtime_extra = 0;
        let mut size_high = 0;
        let mut file_acl_high = 0;
        let mut obso_faddr = 0;
        let mut projid = 0;
        let mut faddr_ext = 0;

        // Check if we have extended fields
        if data.len() >= 128 {
            extra_isize = read_u16(116);
            checksum = read_u16(118);

            if data.len() >= 156 {
                ctime_extra = read_u32(120);
                mtime_extra = read_u32(124);
                atime_extra = read_u32(128);
                crtime = read_u32(132);
                crtime_extra = read_u32(136);
                size_high = read_u32(140);
                file_acl_high = read_u32(144);
                obso_faddr = read_u32(148);

                if data.len() >= 160 {
                    projid = read_u32(152);
                    faddr_ext = read_u32(156);
                }
            }
        }

        // Combine high and low parts for 64-bit values
        let size = ((size_high as u64) << 32) | (size_lo as u64);
        let blocks = blocks_lo as u64; // blocks is actually 32-bit in ext4

        Ok(Self {
            ino,
            mode: InodeMode::from_bits_truncate(mode),
            uid,
            size,
            atime,
            ctime,
            mtime,
            dtime,
            gid,
            links_count,
            blocks,
            flags,
            version,
            file_acl,
            dir_acl,
            faddr,
            block,
            generation,
            faddr_ext,
            file_acl_high,
            size_high,
            obso_faddr,
            extra_isize,
            checksum,
            ctime_extra,
            mtime_extra,
            atime_extra,
            crtime,
            crtime_extra,
            projid,
        })
    }

    /// Get inode type
    pub fn inode_type(&self) -> InodeType {
        let mode_type = self.mode & InodeMode::IFMT;
        if mode_type == InodeMode::IFDIR {
            InodeType::Directory
        } else if mode_type == InodeMode::IFCHR {
            InodeType::CharDevice
        } else if mode_type == InodeMode::IFBLK {
            InodeType::BlockDevice
        } else if mode_type == InodeMode::IFREG {
            InodeType::File
        } else if mode_type == InodeMode::IFIFO {
            InodeType::Fifo
        } else if mode_type == InodeMode::IFSOCK {
            InodeType::Socket
        } else if mode_type == InodeMode::IFLNK {
            InodeType::SymLink
        } else {
            InodeType::File // Default to file
        }
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.inode_type() == InodeType::Directory
    }

    /// Check if this is a regular file
    pub fn is_file(&self) -> bool {
        self.inode_type() == InodeType::File
    }

    /// Check if this is a symbolic link
    pub fn is_symlink(&self) -> bool {
        self.inode_type() == InodeType::SymLink
    }

    /// Get file permissions
    pub fn permissions(&self) -> u16 {
        (self.mode
            & (InodeMode::IRUSR
                | InodeMode::IWUSR
                | InodeMode::IXUSR
                | InodeMode::IRGRP
                | InodeMode::IWGRP
                | InodeMode::IXGRP
                | InodeMode::IROTH
                | InodeMode::IWOTH
                | InodeMode::IXOTH))
            .bits()
    }

    /// Get block number for a given file offset
    pub fn get_block_number<D>(
        &self,
        offset: u64,
        block_size: u32,
        fs: &crate::Ext4FileSystem<D>,
    ) -> Ext4Result<u32>
    where
        D: axdriver_block::BlockDriverOps,
    {
        let block_index = offset / block_size as u64;

        // Check if filesystem uses extents
        if fs.superblock.feature_incompat() & 0x0040 != 0 {
            // EXT4_FEATURE_INCOMPAT_EXTENTS - use extent tree
            crate::extent::find_block_in_extent_tree(fs, &self.block, block_index as u32)
        } else {
            // Traditional block mapping
            if block_index < 12 {
                // Direct block
                let block_num = self.block[block_index as usize];
                // Validate block number
                if block_num == 0 || block_num >= fs.superblock().blocks_count() as u32 {
                    return Ok(0);
                }
                Ok(block_num)
            } else if block_index < 12 + (block_size as u64 / 4) {
                // Singly indirect block
                let indirect_index = block_index - 12;
                if self.block[12] == 0 {
                    return Ok(0);
                }
                let block_num = self.get_indirect_block(self.block[12], indirect_index as u32, block_size, fs)?;
                // Validate block number
                if block_num == 0 || block_num >= fs.superblock().blocks_count() as u32 {
                    return Ok(0);
                }
                Ok(block_num)
            } else if block_index
                < 12 + (block_size as u64 / 4) + ((block_size as u64 / 4) * (block_size as u64 / 4))
            {
                // Doubly indirect block
                let doubly_index = block_index - 12 - (block_size as u64 / 4);
                let first_level = doubly_index / (block_size as u64 / 4);
                let second_level = doubly_index % (block_size as u64 / 4);
                
                if self.block[13] == 0 {
                    return Ok(0);
                }
                
                let indirect_block =
                    self.get_indirect_block(self.block[13], first_level as u32, block_size, fs)?;
                if indirect_block == 0 {
                    return Ok(0);
                }
                
                let block_num = self.get_indirect_block(indirect_block, second_level as u32, block_size, fs)?;
                // Validate block number
                if block_num == 0 || block_num >= fs.superblock().blocks_count() as u32 {
                    return Ok(0);
                }
                Ok(block_num)
            } else {
                // Triply indirect block
                let triply_index = block_index
                    - 12
                    - (block_size as u64 / 4)
                    - ((block_size as u64 / 4) * (block_size as u64 / 4));
                let first_level = triply_index / ((block_size as u64 / 4) * (block_size as u64 / 4));
                let remaining = triply_index % ((block_size as u64 / 4) * (block_size as u64 / 4));
                let second_level = remaining / (block_size as u64 / 4);
                let third_level = remaining % (block_size as u64 / 4);

                if self.block[14] == 0 {
                    return Ok(0);
                }

                let indirect_block =
                    self.get_indirect_block(self.block[14], first_level as u32, block_size, fs)?;
                if indirect_block == 0 {
                    return Ok(0);
                }
                
                let doubly_indirect =
                    self.get_indirect_block(indirect_block, second_level as u32, block_size, fs)?;
                if doubly_indirect == 0 {
                    return Ok(0);
                }
                
                let block_num = self.get_indirect_block(doubly_indirect, third_level as u32, block_size, fs)?;
                // Validate block number
                if block_num == 0 || block_num >= fs.superblock().blocks_count() as u32 {
                    return Ok(0);
                }
                Ok(block_num)
            }
        }
    }

    /// Get block from indirect block
    fn get_indirect_block<D>(
        &self,
        indirect_block: u32,
        index: u32,
        block_size: u32,
        fs: &crate::Ext4FileSystem<D>,
    ) -> Ext4Result<u32>
    where
        D: axdriver_block::BlockDriverOps,
    {
        if indirect_block == 0 {
            return Ok(0); // Sparse block
        }

        let mut buf = vec![0u8; block_size as usize];
        fs.read_block(indirect_block, &mut buf)?;

        let offset = index as usize * 4;
        if offset + 4 > buf.len() {
            return Err(crate::Ext4Error::InvalidInput);
        }

        let block_num = (buf[offset] as u32)
            | ((buf[offset + 1] as u32) << 8)
            | ((buf[offset + 2] as u32) << 16)
            | ((buf[offset + 3] as u32) << 24);

        Ok(block_num)
    }

    /// Set block in indirect block
    fn set_indirect_block<D>(
        &mut self,
        indirect_block: u32,
        index: u32,
        block_num: u32,
        block_size: u32,
        fs: &mut crate::Ext4FileSystem<D>,
    ) -> Ext4Result<()>
    where
        D: axdriver_block::BlockDriverOps,
    {
        if indirect_block == 0 {
            return Err(crate::Ext4Error::InvalidInput);
        }

        let mut buf = vec![0u8; block_size as usize];
        fs.read_block(indirect_block, &mut buf)?;

        let offset = index as usize * 4;
        if offset + 4 > buf.len() {
            return Err(crate::Ext4Error::InvalidInput);
        }

        // Write the block number in little-endian format
        buf[offset] = (block_num & 0xFF) as u8;
        buf[offset + 1] = ((block_num >> 8) & 0xFF) as u8;
        buf[offset + 2] = ((block_num >> 16) & 0xFF) as u8;
        buf[offset + 3] = ((block_num >> 24) & 0xFF) as u8;

        fs.write_block(indirect_block, &buf)?;
        Ok(())
    }

    /// Set block number for a given block index, handling indirect blocks
    pub fn set_block<D>(
        &mut self,
        block_index: u64,
        block_num: u32,
        block_size: u32,
        fs: &mut crate::Ext4FileSystem<D>,
    ) -> Ext4Result<()>
    where
        D: axdriver_block::BlockDriverOps,
    {
        if block_index < 12 {
            // Direct block
            self.block[block_index as usize] = block_num;
            Ok(())
        } else if block_index < 12 + (block_size as u64 / 4) {
            // Singly indirect block
            let indirect_index = block_index - 12;
            if self.block[12] == 0 {
                // Allocate indirect block if needed
                let new_indirect = fs.alloc_block()?;
                self.block[12] = new_indirect;
                // Initialize the indirect block with zeros
                let zero_buf = vec![0u8; block_size as usize];
                fs.write_block(new_indirect, &zero_buf)?;
            }
            self.set_indirect_block(
                self.block[12],
                indirect_index as u32,
                block_num,
                block_size,
                fs,
            )
        } else if block_index
            < 12 + (block_size as u64 / 4) + ((block_size as u64 / 4) * (block_size as u64 / 4))
        {
            // Doubly indirect block
            let doubly_index = block_index - 12 - (block_size as u64 / 4);
            let first_level = doubly_index / (block_size as u64 / 4);
            let second_level = doubly_index % (block_size as u64 / 4);

            if self.block[13] == 0 {
                // Allocate doubly indirect block if needed
                let new_doubly = fs.alloc_block()?;
                self.block[13] = new_doubly;
                // Initialize the doubly indirect block with zeros
                let zero_buf = vec![0u8; block_size as usize];
                fs.write_block(new_doubly, &zero_buf)?;
            }

            // Get or allocate the singly indirect block
            let indirect_block =
                self.get_indirect_block(self.block[13], first_level as u32, block_size, fs)?;
            if indirect_block == 0 {
                // Allocate singly indirect block if needed
                let new_indirect = fs.alloc_block()?;
                self.set_indirect_block(
                    self.block[13],
                    first_level as u32,
                    new_indirect,
                    block_size,
                    fs,
                )?;
                // Initialize the indirect block with zeros
                let zero_buf = vec![0u8; block_size as usize];
                fs.write_block(new_indirect, &zero_buf)?;
                self.set_indirect_block(
                    new_indirect,
                    second_level as u32,
                    block_num,
                    block_size,
                    fs,
                )
            } else {
                self.set_indirect_block(
                    indirect_block,
                    second_level as u32,
                    block_num,
                    block_size,
                    fs,
                )
            }
        } else {
            // Triply indirect block
            let triply_index = block_index
                - 12
                - (block_size as u64 / 4)
                - ((block_size as u64 / 4) * (block_size as u64 / 4));
            let first_level = triply_index / ((block_size as u64 / 4) * (block_size as u64 / 4));
            let remaining = triply_index % ((block_size as u64 / 4) * (block_size as u64 / 4));
            let second_level = remaining / (block_size as u64 / 4);
            let third_level = remaining % (block_size as u64 / 4);

            if self.block[14] == 0 {
                // Allocate triply indirect block if needed
                let new_triply = fs.alloc_block()?;
                self.block[14] = new_triply;
                // Initialize the triply indirect block with zeros
                let zero_buf = vec![0u8; block_size as usize];
                fs.write_block(new_triply, &zero_buf)?;
            }

            // Get or allocate the doubly indirect block
            let doubly_block =
                self.get_indirect_block(self.block[14], first_level as u32, block_size, fs)?;
            let doubly_indirect = if doubly_block == 0 {
                // Allocate doubly indirect block if needed
                let new_doubly = fs.alloc_block()?;
                self.set_indirect_block(
                    self.block[14],
                    first_level as u32,
                    new_doubly,
                    block_size,
                    fs,
                )?;
                // Initialize the doubly indirect block with zeros
                let zero_buf = vec![0u8; block_size as usize];
                fs.write_block(new_doubly, &zero_buf)?;
                new_doubly
            } else {
                doubly_block
            };

            // Get or allocate the singly indirect block
            let singly_block =
                self.get_indirect_block(doubly_indirect, second_level as u32, block_size, fs)?;
            let singly_indirect = if singly_block == 0 {
                // Allocate singly indirect block if needed
                let new_singly = fs.alloc_block()?;
                self.set_indirect_block(
                    doubly_indirect,
                    second_level as u32,
                    new_singly,
                    block_size,
                    fs,
                )?;
                // Initialize the singly indirect block with zeros
                let zero_buf = vec![0u8; block_size as usize];
                fs.write_block(new_singly, &zero_buf)?;
                new_singly
            } else {
                singly_block
            };

            self.set_indirect_block(
                singly_indirect,
                third_level as u32,
                block_num,
                block_size,
                fs,
            )
        }
    }

    /// Get the number of blocks this inode uses
    pub fn block_count(&self, block_size: u32) -> u64 {
        (self.size + block_size as u64 - 1) / block_size as u64
    }

    /// Create a new inode with default values
    pub fn new(ino: u32) -> Self {
        Self {
            ino,
            mode: InodeMode::IFREG,
            uid: 0,
            size: 0,
            atime: 0,
            ctime: 0,
            mtime: 0,
            dtime: 0,
            gid: 0,
            links_count: 1,
            blocks: 0,
            flags: 0,
            version: 0,
            file_acl: 0,
            dir_acl: 0,
            faddr: 0,
            block: [0; 15],
            generation: 0,
            faddr_ext: 0,
            file_acl_high: 0,
            size_high: 0,
            obso_faddr: 0,
            extra_isize: 0,
            checksum: 0,
            ctime_extra: 0,
            mtime_extra: 0,
            atime_extra: 0,
            crtime: 0,
            crtime_extra: 0,
            projid: 0,
        }
    }

    /// Convert inode to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = vec![0u8; 256]; // Maximum inode size

        // Helper function to write little-endian values
        let write_u16 = |data: &mut [u8], offset: usize, value: u16| {
            data[offset] = (value & 0xFF) as u8;
            data[offset + 1] = ((value >> 8) & 0xFF) as u8;
        };

        let write_u32 = |data: &mut [u8], offset: usize, value: u32| {
            data[offset] = (value & 0xFF) as u8;
            data[offset + 1] = ((value >> 8) & 0xFF) as u8;
            data[offset + 2] = ((value >> 16) & 0xFF) as u8;
            data[offset + 3] = ((value >> 24) & 0xFF) as u8;
        };

        // Write basic inode fields
        write_u16(&mut data, 0, self.mode.bits());
        write_u16(&mut data, 2, self.uid);
        write_u32(&mut data, 4, self.size as u32);
        write_u32(&mut data, 8, self.atime);
        write_u32(&mut data, 12, self.ctime);
        write_u32(&mut data, 16, self.mtime);
        write_u32(&mut data, 20, self.dtime);
        write_u16(&mut data, 24, self.gid);
        write_u16(&mut data, 26, self.links_count);
        write_u32(&mut data, 28, self.blocks as u32);
        write_u32(&mut data, 32, self.flags);
        write_u32(&mut data, 36, self.version);

        // Write block pointers
        for i in 0..15 {
            write_u32(&mut data, 40 + i * 4, self.block[i]);
        }

        write_u32(&mut data, 100, self.generation);
        write_u32(&mut data, 104, self.file_acl);
        write_u32(&mut data, 108, self.dir_acl);
        write_u32(&mut data, 112, self.faddr);

        // Write extended fields
        write_u16(&mut data, 116, self.extra_isize);
        write_u16(&mut data, 118, self.checksum);
        write_u32(&mut data, 120, self.ctime_extra);
        write_u32(&mut data, 124, self.mtime_extra);
        write_u32(&mut data, 128, self.atime_extra);
        write_u32(&mut data, 132, self.crtime);
        write_u32(&mut data, 136, self.crtime_extra);
        write_u32(&mut data, 140, self.size_high);
        write_u32(&mut data, 144, self.file_acl_high);
        write_u32(&mut data, 148, self.obso_faddr);

        // Don't truncate - we need full 256 bytes for ext4 inodes
        // data.truncate(128 + self.extra_isize as usize);
        data
    }
}

impl core::fmt::Debug for Inode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inode")
            .field("ino", &self.ino)
            .field("mode", &self.mode.bits())
            .field("uid", &self.uid)
            .field("size", &self.size)
            .field("atime", &self.atime)
            .field("ctime", &self.ctime)
            .field("mtime", &self.mtime)
            .field("dtime", &self.dtime)
            .field("gid", &self.gid)
            .field("links_count", &self.links_count)
            .field("blocks", &self.blocks)
            .field("flags", &self.flags)
            .field("version", &self.version)
            .field("file_acl", &self.file_acl)
            .field("dir_acl", &self.dir_acl)
            .field("faddr", &self.faddr)
            .field("block", &self.block)
            .field("generation", &self.generation)
            .field("faddr_ext", &self.faddr_ext)
            .field("file_acl_high", &self.file_acl_high)
            .field("size_high", &self.size_high)
            .field("obso_faddr", &self.obso_faddr)
            .field("extra_isize", &self.extra_isize)
            .field("checksum", &self.checksum)
            .field("ctime_extra", &self.ctime_extra)
            .field("mtime_extra", &self.mtime_extra)
            .field("atime_extra", &self.atime_extra)
            .field("crtime", &self.crtime)
            .field("crtime_extra", &self.crtime_extra)
            .field("projid", &self.projid)
            .finish()
    }
}
