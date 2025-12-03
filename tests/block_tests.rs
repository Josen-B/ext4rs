//! Tests for block read/write operations

mod common;
use common::MockBlockDevice;

#[test]
fn test_mock_block_device() {
    // Create a mock block device
    let mut device = MockBlockDevice::new(1024, 2048); // 1024 byte blocks, 2048 blocks
    
    // Test device properties
    assert_eq!(device.block_size(), 1024);
    assert_eq!(device.num_blocks(), 2048);
    assert_eq!(device.size(), 1024 * 2048);
    
    // Test block read/write
    let test_data = b"Hello, ext4 block test!";
    let mut read_buffer = vec![0u8; test_data.len()];
    
    // Write test data to block 100
    device.write_block(100, test_data).expect("Failed to write block");
    
    // Read the data back
    device.read_block(100, &mut read_buffer).expect("Failed to read block");
    
    // Verify the data
    assert_eq!(test_data, &read_buffer[..], "Block read/write data mismatch");
}

#[test]
fn test_multiple_block_operations() {
    // Create a mock block device
    let mut device = MockBlockDevice::new(1024, 2048);
    
    // Test multiple block operations
    let test_blocks = [10, 50, 100, 500, 1000];
    let test_data = b"Block test data for multiple blocks";
    
    for &block_num in &test_blocks {
        // Write test data
        device.write_block(block_num, test_data).expect("Failed to write block");
        
        // Read back and verify
        let mut read_buffer = vec![0u8; test_data.len()];
        device.read_block(block_num, &mut read_buffer).expect("Failed to read block");
        
        assert_eq!(test_data, &read_buffer[..], "Data mismatch for block {}", block_num);
    }
}

#[test]
fn test_block_boundaries() {
    // Create a mock block device
    let mut device = MockBlockDevice::new(1024, 2048);
    
    // Test reading/writing at block boundaries
    let first_block = 0;
    let last_block = 2047;
    
    let test_data = vec![0xABu8; 1024];
    
    // Test first block
    device.write_block(first_block, &test_data).expect("Failed to write first block");
    let mut read_buffer = vec![0u8; 1024];
    device.read_block(first_block, &mut read_buffer).expect("Failed to read first block");
    assert_eq!(test_data, read_buffer, "First block data mismatch");
    
    // Test last block
    device.write_block(last_block, &test_data).expect("Failed to write last block");
    device.read_block(last_block, &mut read_buffer).expect("Failed to read last block");
    assert_eq!(test_data, read_buffer, "Last block data mismatch");
}

#[test]
fn test_invalid_block_access() {
    // Create a mock block device
    let mut device = MockBlockDevice::new(1024, 2048);
    
    // Test invalid block access
    let invalid_block = 2048; // One beyond the total blocks
    let test_data = b"Test data";
    
    // Should fail when writing to invalid block
    assert!(device.write_block(invalid_block, test_data).is_err(), "Writing to invalid block should fail");
    
    // Should fail when reading from invalid block
    let mut read_buffer = vec![0u8; test_data.len()];
    assert!(device.read_block(invalid_block, &mut read_buffer).is_err(), "Reading from invalid block should fail");
}