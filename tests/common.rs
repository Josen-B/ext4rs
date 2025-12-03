//! Common utilities for testing

extern crate alloc;

use alloc::vec::Vec;

/// A simple mock block device for testing
pub struct MockBlockDevice {
    data: Vec<u8>,
    block_size: u32,
    total_blocks: u32,
}

impl MockBlockDevice {
    /// Create a new mock block device with the given size
    pub fn new(block_size: u32, total_blocks: u32) -> Self {
        let size = (block_size * total_blocks) as usize;
        Self {
            data: vec![0u8; size],
            block_size,
            total_blocks,
        }
    }

    /// Get the total size of the device
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Write data directly to the device (for setup)
    pub fn write_direct(&mut self, offset: usize, data: &[u8]) {
        let end = offset + data.len();
        assert!(end <= self.data.len(), "Write beyond device bounds");
        self.data[offset..end].copy_from_slice(data);
    }

    /// Read data directly from the device (for verification)
    pub fn read_direct(&self, offset: usize, buf: &mut [u8]) {
        let end = offset + buf.len();
        assert!(end <= self.data.len(), "Read beyond device bounds");
        buf.copy_from_slice(&self.data[offset..end]);
    }

    /// Read a block from the device
    pub fn read_block(&mut self, block_id: u32, buf: &mut [u8]) -> Result<(), &'static str> {
        if block_id >= self.total_blocks {
            return Err("Invalid block ID");
        }

        let offset = (block_id * self.block_size) as usize;
        let end = offset + buf.len();
        
        if end > self.data.len() {
            return Err("Read beyond device bounds");
        }

        buf.copy_from_slice(&self.data[offset..end]);
        Ok(())
    }

    /// Write a block to the device
    pub fn write_block(&mut self, block_id: u32, buf: &[u8]) -> Result<(), &'static str> {
        if block_id >= self.total_blocks {
            return Err("Invalid block ID");
        }

        let offset = (block_id * self.block_size) as usize;
        let end = offset + buf.len();
        
        if end > self.data.len() {
            return Err("Write beyond device bounds");
        }

        self.data[offset..end].copy_from_slice(buf);
        Ok(())
    }

    /// Get the number of blocks
    pub fn num_blocks(&self) -> u32 {
        self.total_blocks
    }

    /// Get the block size
    pub fn block_size(&self) -> u32 {
        self.block_size
    }
}

/// Create a minimal ext4 superblock for testing
pub fn create_test_superblock() -> Vec<u8> {
    let mut sb = vec![0u8; 1024]; // Standard superblock size
    
    // Magic number (ext4 signature)
    sb[56..60].copy_from_slice(&0xEF53u16.to_le_bytes());
    
    // Number of inodes
    sb[4..8].copy_from_slice(&128u32.to_le_bytes());
    
    // Number of blocks
    sb[8..12].copy_from_slice(&1024u32.to_le_bytes());
    
    // Blocks per group
    sb[32..36].copy_from_slice(&8192u32.to_le_bytes());
    
    // Inodes per group
    sb[40..44].copy_from_slice(&128u32.to_le_bytes());
    
    // First data block
    sb[20..24].copy_from_slice(&1u32.to_le_bytes());
    
    // Block size (1024 << 0 = 1024)
    sb[24..28].copy_from_slice(&0u32.to_le_bytes());
    
    // Inode size
    sb[88..92].copy_from_slice(&128u32.to_le_bytes());
    
    // Revision level
    sb[76..80].copy_from_slice(&1u32.to_le_bytes());
    
    sb
}

/// Create a minimal block group descriptor for testing
pub fn create_test_block_group_descriptor() -> Vec<u8> {
    let mut bgd = vec![0u8; 32]; // Standard block group descriptor size
    
    // Block bitmap
    bgd[0..4].copy_from_slice(&3u32.to_le_bytes());
    
    // Inode bitmap
    bgd[4..8].copy_from_slice(&4u32.to_le_bytes());
    
    // Inode table
    bgd[8..12].copy_from_slice(&5u32.to_le_bytes());
    
    // Free blocks count
    bgd[12..16].copy_from_slice(&1019u32.to_le_bytes());
    
    // Free inodes count
    bgd[16..20].copy_from_slice(&126u32.to_le_bytes());
    
    // Used directories count
    bgd[20..24].copy_from_slice(&2u32.to_le_bytes());
    
    bgd
}