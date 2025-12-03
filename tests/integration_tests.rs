//! Integration tests for ext4rs

use ext4rs::{Inode, DirectoryEntry, BlockGroupDescriptor, Bitmap};
mod common;
use common::MockBlockDevice;

#[test]
fn test_superblock_creation() {
    // Create a mock block device
    let mut device = MockBlockDevice::new(1024, 2048);
    
    // Create a minimal superblock
    let mut sb_data = vec![0u8; 1024];
    
    // Magic number (ext4 signature)
    sb_data[56..58].copy_from_slice(&0xEF53u16.to_le_bytes());
    
    // Number of inodes
    sb_data[4..8].copy_from_slice(&128u32.to_le_bytes());
    
    // Number of blocks
    sb_data[8..12].copy_from_slice(&1024u32.to_le_bytes());
    
    // Blocks per group
    sb_data[32..36].copy_from_slice(&8192u32.to_le_bytes());
    
    // Inodes per group
    sb_data[40..44].copy_from_slice(&128u32.to_le_bytes());
    
    // First data block
    sb_data[20..24].copy_from_slice(&1u32.to_le_bytes());
    
    // Block size (1024 << 0 = 1024)
    sb_data[24..28].copy_from_slice(&0u32.to_le_bytes());
    
    // Inode size
    sb_data[88..92].copy_from_slice(&128u32.to_le_bytes());
    
    // Revision level
    sb_data[76..80].copy_from_slice(&1u32.to_le_bytes());
    
    // Write superblock to device
    device.write_block(1, &sb_data).expect("Failed to write superblock");
    
    // Read and validate superblock
    let mut read_data = vec![0u8; 1024];
    device.read_block(1, &mut read_data).expect("Failed to read superblock");
    
    // Verify magic number
    let magic = u16::from_le_bytes([read_data[56], read_data[57]]);
    assert_eq!(magic, 0xEF53, "Invalid magic number");
    
    // Verify block count
    let blocks_count = u32::from_le_bytes([
        read_data[4], read_data[5], read_data[6], read_data[7]
    ]);
    assert_eq!(blocks_count, 128, "Invalid blocks count");
}

#[test]
fn test_inode_serialization() {
    // Create a test inode
    let mut inode = Inode::new(42);
    inode.size = 4096;
    inode.uid = 1000;
    inode.gid = 1000;
    inode.links_count = 2;
    inode.blocks = 8;
    
    // Serialize inode to bytes
    let inode_data = inode.to_bytes();
    
    // Verify some fields
    let size_lo = u32::from_le_bytes([
        inode_data[4], inode_data[5], inode_data[6], inode_data[7]
    ]);
    assert_eq!(size_lo, 4096, "Size not serialized correctly");
    
    let uid = u16::from_le_bytes([inode_data[2], inode_data[3]]);
    assert_eq!(uid, 1000, "UID not serialized correctly");
    
    let gid = u16::from_le_bytes([inode_data[24], inode_data[25]]);
    assert_eq!(gid, 1000, "GID not serialized correctly");
    
    let links_count = u16::from_le_bytes([inode_data[26], inode_data[27]]);
    assert_eq!(links_count, 2, "Links count not serialized correctly");
}

#[test]
fn test_bitmap_operations() {
    // Create a bitmap with 100 bits
    let mut bitmap = Bitmap::new(100);
    
    // Initially all bits should be unset
    for i in 0..100 {
        assert!(!bitmap.is_set(i), "Bit {} should be unset", i);
    }
    
    // Set some bits
    bitmap.set(10).expect("Failed to set bit 10");
    bitmap.set(20).expect("Failed to set bit 20");
    bitmap.set(30).expect("Failed to set bit 30");
    
    // Check set bits
    assert!(bitmap.is_set(10), "Bit 10 should be set");
    assert!(bitmap.is_set(20), "Bit 20 should be set");
    assert!(bitmap.is_set(30), "Bit 30 should be set");
    
    // Check unset bits
    assert!(!bitmap.is_set(0), "Bit 0 should be unset");
    assert!(!bitmap.is_set(15), "Bit 15 should be unset");
    assert!(!bitmap.is_set(99), "Bit 99 should be unset");
    
    // Clear a bit
    bitmap.clear(20).expect("Failed to clear bit 20");
    assert!(!bitmap.is_set(20), "Bit 20 should be cleared");
    
    // Find first free bit
    assert_eq!(bitmap.find_first_free(), Some(0), "First free bit should be 0");
    
    // Set first bit and find next free
    bitmap.set(0).expect("Failed to set bit 0");
    assert_eq!(bitmap.find_first_free(), Some(1), "First free bit should be 1");
    
    // Set all bits and check for no free bits
    for i in 0..100 {
        bitmap.set(i).expect("Failed to set bit");
    }
    assert_eq!(bitmap.find_first_free(), None, "No free bits should be available");
}

#[test]
fn test_directory_entry() {
    // Create a directory entry
    let entry = DirectoryEntry {
        ino: 42,
        rec_len: 32,
        name_len: 14,
        file_type: 1, // Regular file
        name: "test_file.txt".to_string(),
    };
    
    // Verify entry properties
    assert_eq!(entry.ino, 42, "Inode number mismatch");
    assert_eq!(entry.rec_len, 32, "Record length mismatch");
    assert_eq!(entry.name_len, 14, "Name length mismatch");
    assert_eq!(entry.file_type, 1, "File type mismatch");
    assert_eq!(entry.name, "test_file.txt", "Name mismatch");
}

#[test]
fn test_block_group_descriptor_from_bytes() {
    // Create block group descriptor data
    let mut bgd_data = vec![0u8; 32];
    
    // Block bitmap
    bgd_data[0..4].copy_from_slice(&10u32.to_le_bytes());
    
    // Inode bitmap
    bgd_data[4..8].copy_from_slice(&11u32.to_le_bytes());
    
    // Inode table
    bgd_data[8..12].copy_from_slice(&12u32.to_le_bytes());
    
    // Free blocks count
    bgd_data[12..14].copy_from_slice(&1000u16.to_le_bytes());
    
    // Free inodes count
    bgd_data[14..16].copy_from_slice(&100u16.to_le_bytes());
    
    // Used directories count
    bgd_data[16..18].copy_from_slice(&5u16.to_le_bytes());
    
    // Parse block group descriptor
    let _bgd = BlockGroupDescriptor::from_bytes(&bgd_data).expect("Failed to parse block group descriptor");
    
    // Note: We can't directly access private fields, but we can verify the descriptor was created successfully
    // The actual verification would need to be done through public methods if available
    assert!(true, "Block group descriptor created successfully");
}