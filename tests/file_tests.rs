//! Tests for file read/write operations

use ext4rs::{File, Inode};

#[test]
fn test_file_creation() {
    // Create a test inode
    let inode = Inode::new(2); // Inode number 2
    let file = File::new(inode);
    
    // Test file properties
    assert_eq!(file.size(), 0, "New file should have size 0");
    assert_eq!(file.position(), 0, "New file should have position 0");
}

#[test]
fn test_file_seek_operations() {
    // Create a test inode with some size
    let mut inode = Inode::new(2);
    inode.size = 1024; // 1KB file
    
    let mut file = File::new(inode);
    
    // Test seeking
    assert_eq!(file.seek(512).unwrap(), 512, "Seek to 512 should return 512");
    assert_eq!(file.position(), 512, "Position should be 512 after seek");
    
    // Test seeking from current position
    assert_eq!(file.seek_from_current(100).unwrap(), 612, "Seek from current should return 612");
    assert_eq!(file.position(), 612, "Position should be 612 after seek from current");
    
    // Test seeking from end
    assert_eq!(file.seek_from_end(-100).unwrap(), 924, "Seek from end should return 924");
    assert_eq!(file.position(), 924, "Position should be 924 after seek from end");
    
    // Test invalid seek (beyond file size)
    assert!(file.seek(2048).is_err(), "Seek beyond file size should fail");
}

#[test]
fn test_inode_creation() {
    // Test creating different types of inodes
    let file_inode = Inode::new(1);
    assert_eq!(file_inode.ino, 1);
    assert_eq!(file_inode.size, 0);
    
    let dir_inode = Inode::new(2);
    assert_eq!(dir_inode.ino, 2);
    assert_eq!(dir_inode.size, 0);
    
    // Test modifying inode properties
    let mut inode = Inode::new(3);
    inode.size = 4096;
    inode.uid = 1000;
    inode.gid = 1000;
    inode.links_count = 2;
    
    assert_eq!(inode.size, 4096);
    assert_eq!(inode.uid, 1000);
    assert_eq!(inode.gid, 1000);
    assert_eq!(inode.links_count, 2);
}

#[test]
fn test_file_position_tracking() {
    // Create a test inode with some size
    let mut inode = Inode::new(2);
    inode.size = 2048; // 2KB file
    
    let mut file = File::new(inode);
    
    // Test initial position
    assert_eq!(file.position(), 0);
    
    // Test seeking to various positions
    assert_eq!(file.seek(100).unwrap(), 100);
    assert_eq!(file.position(), 100);
    
    assert_eq!(file.seek(0).unwrap(), 0);
    assert_eq!(file.position(), 0);
    
    assert_eq!(file.seek(2048).unwrap(), 2048);
    assert_eq!(file.position(), 2048);
    
    // Test seeking from current position
    assert_eq!(file.seek_from_current(-500).unwrap(), 1548);
    assert_eq!(file.position(), 1548);
    
    assert_eq!(file.seek_from_current(500).unwrap(), 2048);
    assert_eq!(file.position(), 2048);
    
    // Test seeking from end
    assert_eq!(file.seek_from_end(0).unwrap(), 2048);
    assert_eq!(file.position(), 2048);
    
    assert_eq!(file.seek_from_end(-100).unwrap(), 1948);
    assert_eq!(file.position(), 1948);
    
    assert_eq!(file.seek_from_end(-2048).unwrap(), 0);
    assert_eq!(file.position(), 0);
}