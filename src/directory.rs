use alloc::string::String;
use alloc::vec::Vec;
use log::*;

use crate::{Ext4Error, Ext4Result, InodeType};

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Inode number
    pub ino: u32,
    /// Entry length
    pub rec_len: u16,
    /// Name length
    pub name_len: u8,
    /// File type
    pub file_type: u8,
    /// Name
    pub name: String,
}

impl DirectoryEntry {
    /// Parse directory entry from bytes
    pub fn from_bytes(data: &[u8]) -> Ext4Result<Self> {
        if data.len() < 8 {
            return Err(Ext4Error::InvalidInput);
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

        let read_u8 = |offset: usize| -> u8 { data[offset] };

        let ino = read_u32(0);
        let rec_len = read_u16(4);
        let name_len = read_u8(6);

        // In ext4, file_type might not be present if it's an old filesystem
        let file_type = if data.len() > 7 {
            read_u8(7)
        } else {
            0 // Unknown type, will be determined from inode if needed
        };

        if data.len() < 8 + name_len as usize {
            return Err(Ext4Error::InvalidInput);
        }

        let name_bytes = &data[8..8 + name_len as usize];
        let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| Ext4Error::InvalidInput)?;

        Ok(Self {
            ino,
            rec_len,
            name_len,
            file_type,
            name,
        })
    }

    /// Get the inode type
    pub fn inode_type(&self) -> InodeType {
        match self.file_type {
            1 => InodeType::File,
            2 => InodeType::Directory,
            3 => InodeType::CharDevice,
            4 => InodeType::BlockDevice,
            5 => InodeType::Fifo,
            6 => InodeType::Socket,
            7 => InodeType::SymLink,
            _ => InodeType::File, // Default to file
        }
    }

    /// Get the entry size
    pub fn entry_size(&self) -> usize {
        8 + self.name_len as usize
    }
}

/// Directory iterator
pub struct DirectoryIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> DirectoryIterator<'a> {
    /// Create a new directory iterator
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for DirectoryIterator<'a> {
    type Item = Ext4Result<DirectoryEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let entry_data = &self.data[self.offset..];

        // Check if we have enough data for the header
        if entry_data.len() < 8 {
            return None;
        }

        // Helper function to read little-endian values
        let read_u32 = |offset: usize| -> u32 {
            (entry_data[offset] as u32)
                | ((entry_data[offset + 1] as u32) << 8)
                | ((entry_data[offset + 2] as u32) << 16)
                | ((entry_data[offset + 3] as u32) << 24)
        };

        let read_u16 = |offset: usize| -> u16 {
            (entry_data[offset] as u16) | ((entry_data[offset + 1] as u16) << 8)
        };

        // Read the inode number
        let ino = read_u32(0);

        // If inode is 0, this is an unused entry, skip it
        if ino == 0 {
            self.offset += 8; // Skip minimum entry size
            return self.next();
        }

        // Read the record length
        let rec_len = read_u16(4);

        if rec_len == 0 {
            return None;
        }

        // Check if we have enough data for the full entry
        if entry_data.len() < rec_len as usize {
            return None;
        }

        let entry_data = &entry_data[..rec_len as usize];
        let entry = DirectoryEntry::from_bytes(entry_data);

        self.offset += rec_len as usize;

        Some(entry)
    }
}

/// Directory operations
pub struct Directory {
    entries: Vec<DirectoryEntry>,
}

impl Directory {
    /// Create a new directory
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create a directory from raw data
    pub fn from_bytes(data: &[u8]) -> Ext4Result<Self> {
        let mut entries = Vec::new();
        let iter = DirectoryIterator::new(data);

        for entry_result in iter {
            match entry_result {
                Ok(entry) => {
                    // Skip entries with inode 0 (deleted entries)
                    if entry.ino != 0 && !entry.name.is_empty() {
                        debug!("Found directory entry: {} (ino: {})", entry.name, entry.ino);
                        entries.push(entry);
                    }
                }
                Err(e) => {
                    warn!("Error parsing directory entry: {:?}", e);
                    // Continue parsing other entries
                }
            }
        }

        debug!("Parsed {} directory entries", entries.len());
        Ok(Self { entries })
    }

    /// Add an entry to the directory
    pub fn add_entry(&mut self, entry: DirectoryEntry) {
        self.entries.push(entry);
    }

    /// Remove an entry by name
    pub fn remove_entry(&mut self, name: &str) -> Option<DirectoryEntry> {
        let index = self.entries.iter().position(|e| e.name == name)?;
        Some(self.entries.remove(index))
    }

    /// Find an entry by name
    pub fn find_entry(&self, name: &str) -> Option<&DirectoryEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Get all entries
    pub fn entries(&self) -> &[DirectoryEntry] {
        &self.entries
    }

    /// Serialize the directory to bytes
    pub fn to_bytes(&self) -> Ext4Result<Vec<u8>> {
        let mut data = Vec::new();

        for entry in &self.entries {
            let entry_data = self.entry_to_bytes(entry)?;
            data.extend_from_slice(&entry_data);
        }

        Ok(data)
    }

    /// Convert an entry to bytes
    fn entry_to_bytes(&self, entry: &DirectoryEntry) -> Ext4Result<Vec<u8>> {
        let mut data = Vec::new();

        // Inode number (4 bytes)
        data.push((entry.ino & 0xFF) as u8);
        data.push(((entry.ino >> 8) & 0xFF) as u8);
        data.push(((entry.ino >> 16) & 0xFF) as u8);
        data.push(((entry.ino >> 24) & 0xFF) as u8);

        // Record length (2 bytes)
        data.push((entry.rec_len & 0xFF) as u8);
        data.push(((entry.rec_len >> 8) & 0xFF) as u8);

        // Name length (1 byte)
        data.push(entry.name_len);

        // File type (1 byte)
        data.push(entry.file_type);

        // Name
        data.extend_from_slice(entry.name.as_bytes());

        // Padding to align to 4-byte boundary
        while data.len() % 4 != 0 {
            data.push(0);
        }

        Ok(data)
    }
}
