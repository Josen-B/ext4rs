use alloc::vec::Vec;
use log::*;

use crate::{Ext4Error, Ext4Result};

/// Bitmap for tracking allocated blocks/inodes
#[derive(Debug, Clone)]
pub struct Bitmap {
    data: Vec<u8>,
    size: usize,
}

impl Bitmap {
    /// Create a bitmap from bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            size: data.len() * 8,
        }
    }
    
    /// Create a new empty bitmap
    pub fn new(size: usize) -> Self {
        let byte_count = (size + 7) / 8;
        Self {
            data: vec![0; byte_count],
            size,
        }
    }
    
    /// Check if a bit is set
    pub fn is_set(&self, bit: usize) -> bool {
        if bit >= self.size {
            return false;
        }
        
        let byte_index = bit / 8;
        let bit_index = bit % 8;
        (self.data[byte_index] & (1 << bit_index)) != 0
    }
    
    /// Set a bit
    pub fn set(&mut self, bit: usize) -> Ext4Result<()> {
        if bit >= self.size {
            return Err(Ext4Error::InvalidInput);
        }
        
        let byte_index = bit / 8;
        let bit_index = bit % 8;
        self.data[byte_index] |= 1 << bit_index;
        Ok(())
    }
    
    /// Clear a bit
    pub fn clear(&mut self, bit: usize) -> Ext4Result<()> {
        if bit >= self.size {
            return Err(Ext4Error::InvalidInput);
        }
        
        let byte_index = bit / 8;
        let bit_index = bit % 8;
        self.data[byte_index] &= !(1 << bit_index);
        Ok(())
    }
    
    /// Find the first free bit
    pub fn find_first_free(&self) -> Option<usize> {
        for (byte_index, &byte) in self.data.iter().enumerate() {
            if byte != 0xFF {
                for bit_index in 0..8 {
                    let bit = byte_index * 8 + bit_index;
                    if bit < self.size && !self.is_set(bit) {
                        return Some(bit);
                    }
                }
            }
        }
        None
    }
    
    /// Find the first set bit
    pub fn find_first_set(&self) -> Option<usize> {
        for (byte_index, &byte) in self.data.iter().enumerate() {
            if byte != 0 {
                for bit_index in 0..8 {
                    let bit = byte_index * 8 + bit_index;
                    if bit < self.size && self.is_set(bit) {
                        return Some(bit);
                    }
                }
            }
        }
        None
    }
    
    /// Count the number of free bits
    pub fn count_free(&self) -> usize {
        let mut count = 0;
        for &byte in &self.data {
            count += (!byte).count_ones() as usize;
        }
        // Adjust for the last byte if it's not fully used
        let remainder = self.size % 8;
        if remainder != 0 {
            let mask = (1 << remainder) - 1;
            count -= (self.data[self.data.len() - 1] & !mask).count_ones() as usize;
        }
        count
    }
    
    /// Count the number of set bits
    pub fn count_set(&self) -> usize {
        self.size - self.count_free()
    }
    
    /// Get the bitmap data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
    
    /// Get the bitmap size in bits
    pub fn size(&self) -> usize {
        self.size
    }
}