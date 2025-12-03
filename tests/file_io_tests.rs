//! File I/O operations tests

extern crate alloc;
use ext4rs::{File, Inode, InodeMode};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_creation() {
        // Create a test inode
        let mut inode = Inode::new(1);
        inode.mode = InodeMode::IFREG;
        inode.size = 1024; // 1KB file size
        
        // Create a file
        let file = File::new(inode);
        
        // Test file properties
        assert_eq!(file.size(), 1024, "File size should be 1024");
        assert_eq!(file.position(), 0, "Initial position should be 0");
    }

    #[test]
    fn test_file_seek_operations() {
        // Create a test inode
        let mut inode = Inode::new(1);
        inode.mode = InodeMode::IFREG;
        inode.size = 100;
        
        let mut file = File::new(inode);
        
        // Test seeking to valid positions
        assert_eq!(file.seek(10).unwrap(), 10, "Should seek to position 10");
        assert_eq!(file.position(), 10, "Position should be 10");
        
        assert_eq!(file.seek(0).unwrap(), 0, "Should seek to position 0");
        assert_eq!(file.position(), 0, "Position should be 0");
        
        assert_eq!(file.seek(100).unwrap(), 100, "Should seek to end of file");
        assert_eq!(file.position(), 100, "Position should be 100");
    }

    #[test]
    fn test_file_seek_beyond_end() {
        // Create a test inode
        let mut inode = Inode::new(1);
        inode.mode = InodeMode::IFREG;
        inode.size = 100;
        
        let mut file = File::new(inode);
        
        // Test seeking beyond file size (should fail)
        let result = file.seek(200);
        assert!(result.is_err(), "Should not be able to seek beyond file end");
    }

    #[test]
    fn test_file_seek_from_current() {
        // Create a test inode
        let mut inode = Inode::new(1);
        inode.mode = InodeMode::IFREG;
        inode.size = 100;
        
        let mut file = File::new(inode);
        
        // Seek to position 50
        file.seek(50).unwrap();
        
        // Seek from current position
        assert_eq!(file.seek_from_current(10).unwrap(), 60, "Should seek to position 60");
        assert_eq!(file.position(), 60, "Position should be 60");
        
        assert_eq!(file.seek_from_current(-20).unwrap(), 40, "Should seek to position 40");
        assert_eq!(file.position(), 40, "Position should be 40");
    }

    #[test]
    fn test_file_seek_from_current_beyond_bounds() {
        // Create a test inode
        let mut inode = Inode::new(1);
        inode.mode = InodeMode::IFREG;
        inode.size = 100;
        
        let mut file = File::new(inode);
        
        // Seek to position 90
        file.seek(90).unwrap();
        
        // Try to seek beyond file size
        let result = file.seek_from_current(20);
        assert!(result.is_err(), "Should not be able to seek beyond file end");
        
        // Try to seek before start
        file.seek(10).unwrap();
        let result = file.seek_from_current(-20);
        assert!(result.is_err(), "Should not be able to seek before start");
    }

    #[test]
    fn test_file_inode_access() {
        // Create a test inode
        let mut inode = Inode::new(1);
        inode.mode = InodeMode::IFREG;
        inode.size = 1024;
        inode.uid = 1000;
        inode.gid = 1000;
        
        let file = File::new(inode);
        
        // Test inode access
        let file_inode = file.inode();
        assert_eq!(file_inode.ino, 1, "Inode number should be 1");
        assert_eq!(file_inode.size, 1024, "Inode size should be 1024");
        assert_eq!(file_inode.uid, 1000, "Inode uid should be 1000");
        assert_eq!(file_inode.gid, 1000, "Inode gid should be 1000");
        assert_eq!(file_inode.mode, InodeMode::IFREG, "Inode mode should be IFREG");
    }

    #[test]
    fn test_file_position_tracking() {
        // Create a test inode
        let mut inode = Inode::new(1);
        inode.mode = InodeMode::IFREG;
        inode.size = 100;
        
        let mut file = File::new(inode);
        
        // Test initial position
        assert_eq!(file.position(), 0, "Initial position should be 0");
        
        // Test position changes
        file.seek(10).unwrap();
        assert_eq!(file.position(), 10, "Position should be 10");
        
        file.seek_from_current(5).unwrap();
        assert_eq!(file.position(), 15, "Position should be 15");
        
        file.seek_from_current(-5).unwrap();
        assert_eq!(file.position(), 10, "Position should be 10");
        
        file.seek(0).unwrap();
        assert_eq!(file.position(), 0, "Position should be reset to 0");
    }
}