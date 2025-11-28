//! A pure Rust ext4 filesystem implementation
//!
//! This crate provides a complete ext4 filesystem implementation in pure Rust,
//! designed to work with ArceOS and replace the C-based lwext4_rust.

#![no_std]

#[macro_use]
extern crate alloc;

use core::fmt;
use log::*;

mod bitmap;
mod block_group;
mod directory;
mod file;
mod inode;
mod journal;
mod superblock;
mod symlink;

pub use bitmap::Bitmap;
pub use block_group::BlockGroupDescriptor;
pub use directory::{Directory, DirectoryEntry, DirectoryIterator};
pub use file::File;
pub use inode::{Inode, InodeMode, InodeType};
pub use superblock::SuperBlock;

use alloc::vec::Vec;
use axdriver::prelude::*;
use axdriver_block::BlockDriverOps;
use axerrno::{AxError, LinuxError};

/// Ext4 filesystem error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ext4Error {
    /// Invalid filesystem magic number
    InvalidMagic,
    /// Invalid filesystem state
    InvalidState,
    /// Inode not found
    InodeNotFound,
    /// Block not found
    BlockNotFound,
    /// Invalid path
    InvalidPath,
    /// File already exists
    FileExists,
    /// Directory not empty
    DirNotEmpty,
    /// Not a directory
    NotADirectory,
    /// Is a directory
    IsADirectory,
    /// Invalid input
    InvalidInput,
    /// I/O error
    IoError,
    /// No space left on device
    NoSpaceLeft,
    /// Read-only filesystem
    ReadOnly,
    /// Invalid argument
    InvalidArg,
    /// Operation not supported
    NotSupported,
}

impl fmt::Display for Ext4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ext4Error::InvalidMagic => write!(f, "Invalid ext4 magic number"),
            Ext4Error::InvalidState => write!(f, "Invalid filesystem state"),
            Ext4Error::InodeNotFound => write!(f, "Inode not found"),
            Ext4Error::BlockNotFound => write!(f, "Block not found"),
            Ext4Error::InvalidPath => write!(f, "Invalid path"),
            Ext4Error::FileExists => write!(f, "File already exists"),
            Ext4Error::DirNotEmpty => write!(f, "Directory not empty"),
            Ext4Error::NotADirectory => write!(f, "Not a directory"),
            Ext4Error::IsADirectory => write!(f, "Is a directory"),
            Ext4Error::InvalidInput => write!(f, "Invalid input"),
            Ext4Error::IoError => write!(f, "I/O error"),
            Ext4Error::NoSpaceLeft => write!(f, "No space left on device"),
            Ext4Error::ReadOnly => write!(f, "Read-only filesystem"),
            Ext4Error::InvalidArg => write!(f, "Invalid argument"),
            Ext4Error::NotSupported => write!(f, "Operation not supported"),
        }
    }
}

impl From<Ext4Error> for AxError {
    fn from(err: Ext4Error) -> Self {
        let code = match err {
            Ext4Error::InvalidMagic
            | Ext4Error::InvalidState
            | Ext4Error::InvalidPath
            | Ext4Error::InvalidInput
            | Ext4Error::InvalidArg => -(axerrno::LinuxError::EINVAL as i32),
            Ext4Error::InodeNotFound | Ext4Error::BlockNotFound => {
                -(axerrno::LinuxError::ENOENT as i32)
            }
            Ext4Error::FileExists => -(axerrno::LinuxError::EEXIST as i32),
            Ext4Error::DirNotEmpty => -(axerrno::LinuxError::ENOTEMPTY as i32),
            Ext4Error::NotADirectory => -(axerrno::LinuxError::ENOTDIR as i32),
            Ext4Error::IsADirectory => -(axerrno::LinuxError::EISDIR as i32),
            Ext4Error::IoError => -(axerrno::LinuxError::EIO as i32),
            Ext4Error::NoSpaceLeft => -(axerrno::LinuxError::ENOSPC as i32),
            Ext4Error::ReadOnly => -(axerrno::LinuxError::EROFS as i32),
            Ext4Error::NotSupported => -(axerrno::LinuxError::ENOSYS as i32),
        };
        unsafe { core::mem::transmute::<i32, AxError>(code) }
    }
}

/// Result type for ext4 operations
pub type Ext4Result<T> = Result<T, Ext4Error>;

/// Ext4 filesystem implementation
pub struct Ext4FileSystem<D: BlockDriverOps> {
    device: core::cell::RefCell<D>,
    superblock: SuperBlock,
    block_groups: Vec<BlockGroupDescriptor>,
    mount_options: MountOptions,
}

/// Mount options for ext4 filesystem
#[derive(Debug, Clone)]
pub struct MountOptions {
    /// Read-only mount
    pub read_only: bool,
    /// Enable journaling
    pub journaling: bool,
    /// Enable execute permission check
    pub exec_check: bool,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            journaling: true,
            exec_check: false,
        }
    }
}

impl<D: axdriver_block::BlockDriverOps> Ext4FileSystem<D> {
    /// Create a new ext4 filesystem instance
    pub fn new(mut device: D, options: MountOptions) -> Ext4Result<Self> {
        info!("Initializing ext4 filesystem");

        // Read and validate superblock
        let superblock = SuperBlock::read_from_device(&mut device)?;
        superblock.validate()?;

        // Read block group descriptors
        let block_groups = Self::read_block_groups(&mut device, &superblock)?;

        Ok(Self {
            device: core::cell::RefCell::new(device),
            superblock,
            block_groups,
            mount_options: options,
        })
    }

    /// Read block group descriptors
    fn read_block_groups(
        device: &mut D,
        superblock: &SuperBlock,
    ) -> Ext4Result<Vec<BlockGroupDescriptor>> {
        let block_size = superblock.block_size();
        let blocks_count = superblock.blocks_count();
        let blocks_per_group = superblock.blocks_per_group();

        // Handle small filesystems where blocks_count < blocks_per_group
        let groups_count = if blocks_count == 0 {
            0
        } else {
            // Ensure at least one group for non-empty filesystems
            ((blocks_count + blocks_per_group as u64 - 1) / blocks_per_group as u64).max(1)
        };

        let desc_size = if superblock.rev_level() >= 1 { 64 } else { 32 };
        let blocks_per_desc = block_size / desc_size;
        let desc_blocks = (groups_count + blocks_per_desc as u64 - 1) / blocks_per_desc as u64;

        debug!("Reading block groups: blocks_count={}, blocks_per_group={}, groups_count={}, desc_size={}, blocks_per_desc={}, desc_blocks={}", 
                blocks_count, blocks_per_group, groups_count, desc_size, blocks_per_desc, desc_blocks);

        let mut descriptors = Vec::with_capacity(groups_count as usize);
        let mut buf = vec![0u8; block_size as usize];

        for i in 0..desc_blocks {
            let block = (superblock.first_data_block() as u64) + 1 + i;
            debug!("Reading block group descriptor block {}", block);
            device
                .read_block(block, &mut buf)
                .map_err(|_| Ext4Error::IoError)?;

            let base = i * blocks_per_desc as u64;
            for j in 0..blocks_per_desc.min((groups_count - base) as u32) {
                let offset = j * desc_size;
                let desc = BlockGroupDescriptor::from_bytes(
                    &buf[offset as usize..(offset + desc_size) as usize],
                )?;
                debug!(
                    "Block group {}: inode_table={}",
                    descriptors.len(),
                    desc.inode_table()
                );
                descriptors.push(desc);
            }
        }

        debug!("Read {} block group descriptors", descriptors.len());
        Ok(descriptors)
    }

    /// Get the root inode
    pub fn root_inode(&self) -> Ext4Result<Inode> {
        self.get_inode(EXT4_ROOT_INO)
    }

    /// Get the superblock
    pub fn superblock(&self) -> &SuperBlock {
        &self.superblock
    }

    /// Get an inode by number
    pub fn get_inode(&self, ino: u32) -> Ext4Result<Inode> {
        debug!(
            "Getting inode {} with inodes_per_group={}",
            ino,
            self.superblock.inodes_per_group()
        );
        let block_group = (ino - 1) / self.superblock.inodes_per_group();
        let index = (ino - 1) % self.superblock.inodes_per_group();

        debug!(
            "Inode {} -> block_group={}, index={}",
            ino, block_group, index
        );

        if block_group as usize >= self.block_groups.len() {
            error!(
                "Block group {} out of range (total: {})",
                block_group,
                self.block_groups.len()
            );
            return Err(Ext4Error::InodeNotFound);
        }

        let bg_desc = &self.block_groups[block_group as usize];
        let inode_table_block = bg_desc.inode_table();
        let inode_size = self.superblock.inode_size();
        let inodes_per_block = self.superblock.block_size() / inode_size as u32;
        let block_offset = index / inodes_per_block;
        let inode_offset = (index % inodes_per_block) * inode_size as u32;

        debug!(
            "Reading inode table block {} + {} = {}",
            inode_table_block,
            block_offset,
            inode_table_block + block_offset
        );

        let mut buf = vec![0u8; self.superblock.block_size() as usize];
        self.device
            .borrow_mut()
            .read_block((inode_table_block + block_offset) as u64, &mut buf)
            .map_err(|_| Ext4Error::IoError)?;

        debug!(
            "Reading inode at offset {} size {}",
            inode_offset, inode_size
        );
        Inode::from_bytes(
            &buf[inode_offset as usize..(inode_offset + inode_size as u32) as usize],
            ino,
        )
    }

    /// Read a block from the filesystem
    pub fn read_block(&self, block: u32, buf: &mut [u8]) -> Ext4Result<()> {
        if buf.len() != self.superblock.block_size() as usize {
            return Err(Ext4Error::InvalidInput);
        }

        self.device
            .borrow_mut()
            .read_block(block as u64, buf)
            .map_err(|_| Ext4Error::IoError)?;
        Ok(())
    }

    /// Write a block to the filesystem
    pub fn write_block(&self, block: u32, buf: &[u8]) -> Ext4Result<()> {
        if self.mount_options.read_only {
            return Err(Ext4Error::ReadOnly);
        }

        if buf.len() != self.superblock.block_size() as usize {
            return Err(Ext4Error::InvalidInput);
        }

        self.device
            .borrow_mut()
            .write_block(block as u64, buf)
            .map_err(|_| Ext4Error::IoError)?;
        Ok(())
    }

    /// Allocate a new block
    pub fn alloc_block(&self) -> Ext4Result<u32> {
        if self.mount_options.read_only {
            return Err(Ext4Error::ReadOnly);
        }

        // Simple block allocation - find first free block
        for (i, bg) in self.block_groups.iter().enumerate() {
            if bg.free_blocks_count() > 0 {
                let block_bitmap = bg.block_bitmap();
                let mut buf = vec![0u8; self.superblock.block_size() as usize];
                self.read_block(block_bitmap, &mut buf)?;

                let bitmap = Bitmap::from_bytes(&buf);
                if let Some(bit) = bitmap.find_first_free() {
                    let block = i as u32 * self.superblock.blocks_per_group() + bit as u32;
                    return Ok(block);
                }
            }
        }

        Err(Ext4Error::NoSpaceLeft)
    }

    /// Allocate a new inode
    pub fn alloc_inode(&self) -> Ext4Result<u32> {
        if self.mount_options.read_only {
            return Err(Ext4Error::ReadOnly);
        }

        // Simple inode allocation - find first free inode
        for (i, bg) in self.block_groups.iter().enumerate() {
            if bg.free_inodes_count() > 0 {
                let inode_bitmap = bg.inode_bitmap();
                let mut buf = vec![0u8; self.superblock.block_size() as usize];
                self.read_block(inode_bitmap, &mut buf)?;

                let bitmap = Bitmap::from_bytes(&buf);
                if let Some(bit) = bitmap.find_first_free() {
                    let ino = i as u32 * self.superblock.inodes_per_group() + bit as u32 + 1;
                    return Ok(ino);
                }
            }
        }

        Err(Ext4Error::NoSpaceLeft)
    }

    /// Get filesystem statistics
    pub fn stats(&self) -> Ext4Result<FilesystemStats> {
        Ok(FilesystemStats {
            block_size: self.superblock.block_size(),
            total_blocks: self.superblock.blocks_count(),
            free_blocks: self.superblock.free_blocks_count(),
            total_inodes: self.superblock.inodes_count() as u64,
            free_inodes: self.superblock.free_inodes_count() as u64,
        })
    }
}

/// Filesystem statistics
#[derive(Debug, Clone)]
pub struct FilesystemStats {
    pub block_size: u32,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
}

/// Root inode number
pub const EXT4_ROOT_INO: u32 = 2;

/// Invalid inode number
pub const EXT4_BAD_INO: u32 = 1;
