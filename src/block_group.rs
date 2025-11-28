use log::*;

use crate::{Ext4Error, Ext4Result};

/// Block group descriptor
#[derive(Debug, Clone)]
pub struct BlockGroupDescriptor {
    /// Block bitmap
    block_bitmap: u32,
    /// Inode bitmap
    inode_bitmap: u32,
    /// Inode table
    inode_table: u32,
    /// Free blocks count
    free_blocks_count: u16,
    /// Free inodes count
    free_inodes_count: u16,
    /// Used directories count
    used_dirs_count: u16,
    /// Flags
    flags: u16,
    /// Exclude bitmap for snapshots
    exclude_bitmap: u32,
    /// Block bitmap checksum
    block_bitmap_csum: u16,
    /// Inode bitmap checksum
    inode_bitmap_csum: u16,
    /// Unused inode count
    itable_unused: u16,
    /// Checksum
    checksum: u16,
}

impl BlockGroupDescriptor {
    /// Parse block group descriptor from bytes
    pub fn from_bytes(data: &[u8]) -> Ext4Result<Self> {
        if data.len() < 32 {
            return Err(Ext4Error::InvalidInput);
        }
        
        // Helper function to read little-endian values
        let read_u32 = |offset: usize| -> u32 {
            (data[offset] as u32) |
            ((data[offset + 1] as u32) << 8) |
            ((data[offset + 2] as u32) << 16) |
            ((data[offset + 3] as u32) << 24)
        };
        
        let read_u16 = |offset: usize| -> u16 {
            (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
        };
        
        let block_bitmap = read_u32(0);
        let inode_bitmap = read_u32(4);
        let inode_table = read_u32(8);
        let free_blocks_count = read_u16(12);
        let free_inodes_count = read_u16(14);
        let used_dirs_count = read_u16(16);
        let flags = read_u16(18);
        
        // Extended fields (if available)
        let mut exclude_bitmap = 0;
        let mut block_bitmap_csum = 0;
        let mut inode_bitmap_csum = 0;
        let mut itable_unused = 0;
        let mut checksum = 0;
        
        if data.len() >= 64 {
            exclude_bitmap = read_u32(20);
            block_bitmap_csum = read_u16(24);
            inode_bitmap_csum = read_u16(26);
            itable_unused = read_u16(28);
            checksum = read_u16(30);
        }
        
        Ok(Self {
            block_bitmap,
            inode_bitmap,
            inode_table,
            free_blocks_count,
            free_inodes_count,
            used_dirs_count,
            flags,
            exclude_bitmap,
            block_bitmap_csum,
            inode_bitmap_csum,
            itable_unused,
            checksum,
        })
    }
    
    /// Getters
    pub fn block_bitmap(&self) -> u32 { self.block_bitmap }
    pub fn inode_bitmap(&self) -> u32 { self.inode_bitmap }
    pub fn inode_table(&self) -> u32 { self.inode_table }
    pub fn free_blocks_count(&self) -> u16 { self.free_blocks_count }
    pub fn free_inodes_count(&self) -> u16 { self.free_inodes_count }
    pub fn used_dirs_count(&self) -> u16 { self.used_dirs_count }
    pub fn flags(&self) -> u16 { self.flags }
    pub fn exclude_bitmap(&self) -> u32 { self.exclude_bitmap }
    pub fn block_bitmap_csum(&self) -> u16 { self.block_bitmap_csum }
    pub fn inode_bitmap_csum(&self) -> u16 { self.inode_bitmap_csum }
    pub fn itable_unused(&self) -> u16 { self.itable_unused }
    pub fn checksum(&self) -> u16 { self.checksum }
}