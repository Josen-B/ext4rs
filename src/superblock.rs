use alloc::vec::Vec;
use axdriver_block::BlockDriverOps;
use log::*;

use crate::{Ext4Error, Ext4Result};

/// Ext4 superblock structure
#[derive(Debug, Clone)]
pub struct SuperBlock {
    /// Inode count
    inodes_count: u32,
    /// Block count
    blocks_count: u64,
    /// Reserved block count
    reserved_blocks_count: u64,
    /// Free blocks count
    free_blocks_count: u64,
    /// Free inodes count
    free_inodes_count: u32,
    /// First data block
    first_data_block: u32,
    /// Block size
    block_size: u32,
    /// Cluster size
    cluster_size: u32,
    /// Blocks per group
    blocks_per_group: u32,
    /// Clusters per group
    clusters_per_group: u32,
    /// Inodes per group
    inodes_per_group: u32,
    /// Mount time
    mount_time: u32,
    /// Write time
    write_time: u32,
    /// Mount count
    mount_count: u16,
    /// Max mount count
    max_mount_count: u16,
    /// Magic number
    magic: u16,
    /// Filesystem state
    state: u16,
    /// Error behavior
    errors: u16,
    /// Minor revision level
    minor_rev_level: u16,
    /// Last check time
    last_check_time: u32,
    /// Check interval
    check_interval: u32,
    /// Creator OS
    creator_os: u32,
    /// Revision level
    rev_level: u32,
    /// Default reserved uid
    default_reserved_uid: u16,
    /// Default reserved gid
    default_reserved_gid: u16,
    /// First inode
    first_inode: u32,
    /// Inode size
    inode_size: u16,
    /// Block group number of this superblock
    block_group_nr: u16,
    /// Feature compatibility flags
    feature_compat: u32,
    /// Feature incompatibility flags
    feature_incompat: u32,
    /// Feature read-only compatibility flags
    feature_ro_compat: u32,
    /// Filesystem UUID
    uuid: [u8; 16],
    /// Volume name
    volume_name: [u8; 16],
    /// Last mounted directory
    last_mounted: [u8; 64],
    /// Algorithm usage bitmap
    algorithm_usage_bitmap: u32,
    /// Preallocation blocks
    prealloc_blocks: u8,
    /// Preallocation directory blocks
    prealloc_dir_blocks: u8,
    /// Reserved GDT blocks
    reserved_gdt_blocks: u16,
    /// Journal UUID
    journal_uuid: [u8; 16],
    /// Journal inode number
    journal_inum: u32,
    /// Journal device
    journal_dev: u32,
    /// Last orphan inode
    last_orphan: u32,
    /// Hash seed
    hash_seed: [u32; 4],
    /// Default hash version
    def_hash_version: u8,
    /// Journal backup type
    jnl_backup_type: u8,
    /// Descriptor size
    desc_size: u16,
    /// Default mount options
    default_mount_opts: u32,
    /// First metablock block group
    first_meta_bg: u32,
    /// Mkfs time
    mkfs_time: u32,
    /// Journal backup blocks
    jnl_blocks: [u32; 17],
    /// 64-bit support
    blocks_count_hi: u32,
    /// 64-bit support
    reserved_blocks_count_hi: u32,
    /// 64-bit support
    free_blocks_count_hi: u32,
    /// Min extra inode size
    min_extra_isize: u16,
    /// Want extra inode size
    want_extra_isize: u16,
    /// Flags
    flags: u32,
    /// RAID stride
    raid_stride: u16,
    /// Multi-mount protection interval
    mmp_interval: u8,
    /// Multi-mount protection block number
    mmp_block: u64,
    /// RAID stripe width
    raid_stripe_width: u32,
    /// Checksum type
    checksum_type: u8,
    /// Padding
    padding: u8,
    /// Checksum seed
    checksum_seed: u32,
    /// Writable snapshots
    wtime_hi: u16,
    /// Mtime high bits
    mtime_hi: u16,
    /// Mkfs time high bits
    mkfs_time_hi: u16,
    /// Mtime high bits
    awtime_hi: u16,
    /// Checksum of the superblock
    checksum: u32,
}

impl SuperBlock {
    /// Create a new superblock by reading from device
    pub fn read_from_device<D>(device: &mut D) -> Ext4Result<Self>
    where
        D: axdriver_block::BlockDriverOps,
    {
        // The ext4 superblock is always at offset 1024 from the start of the filesystem
        // We need to read blocks that contain this offset
        let block_size = device.block_size();
        let start_block = 1024 / block_size;
        let offset_in_block = 1024 % block_size;

        let mut buf = vec![0u8; 1024]; // Read 1024 bytes for superblock
        let mut temp_buf = vec![0u8; block_size];

        // Read the block that contains the superblock
        device
            .read_block(start_block as u64, &mut temp_buf)
            .map_err(|_| Ext4Error::IoError)?;

        // Copy superblock data from the block
        let remaining = block_size - offset_in_block;
        let to_copy = core::cmp::min(1024, remaining);
        buf[..to_copy].copy_from_slice(&temp_buf[offset_in_block..offset_in_block + to_copy]);

        // If we need more data, read the next block
        if to_copy < 1024 {
            device
                .read_block((start_block + 1) as u64, &mut temp_buf)
                .map_err(|_| Ext4Error::IoError)?;
            let remaining_to_copy = 1024 - to_copy;
            buf[to_copy..].copy_from_slice(&temp_buf[..remaining_to_copy]);
        }

        // Parse the superblock
        Self::from_bytes(&buf)
    }

    /// Parse superblock from bytes
    pub fn from_bytes(data: &[u8]) -> Ext4Result<Self> {
        if data.len() < 1024 {
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

        let read_bytes =
            |offset: usize, len: usize| -> Vec<u8> { data[offset..offset + len].to_vec() };

        // Read superblock fields according to ext4 specification
        let inodes_count = read_u32(0);
        let blocks_count_lo = read_u32(4);
        let reserved_blocks_count_lo = read_u32(8);
        let free_blocks_count_lo = read_u32(12);
        let free_inodes_count = read_u32(16);
        let first_data_block = read_u32(20);
        let log_block_size = read_u32(24);
        let log_cluster_size = read_u32(28);
        let blocks_per_group = read_u32(32);
        let clusters_per_group = read_u32(36);
        let inodes_per_group = read_u32(40);
        let mount_time = read_u32(44);
        let write_time = read_u32(48);
        let mount_count = read_u16(52);
        let max_mount_count = read_u16(54);
        let magic = read_u16(56);
        let state = read_u16(58);
        let errors = read_u16(60);
        let minor_rev_level = read_u16(62);
        let last_check_time = read_u32(64);
        let check_interval = read_u32(68);
        let creator_os = read_u32(72);
        let rev_level = read_u32(76);
        let default_reserved_uid = read_u16(80);
        let default_reserved_gid = read_u16(82);
        let first_inode = read_u32(84);
        let inode_size = read_u16(88);
        let block_group_nr = read_u16(90);
        let feature_compat = read_u32(92);
        let feature_incompat = read_u32(96);
        let feature_ro_compat = read_u32(100);

        let uuid = {
            let bytes = read_bytes(104, 16);
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes);
            arr
        };

        let volume_name = {
            let bytes = read_bytes(120, 16);
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes);
            arr
        };

        let last_mounted = {
            let bytes = read_bytes(136, 64);
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&bytes);
            arr
        };

        let algorithm_usage_bitmap = read_u32(200);
        let prealloc_blocks = read_u8(204);
        let prealloc_dir_blocks = read_u8(205);
        let reserved_gdt_blocks = read_u16(206);

        let journal_uuid = {
            let bytes = read_bytes(208, 16);
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes);
            arr
        };

        let journal_inum = read_u32(224);
        let journal_dev = read_u32(228);
        let last_orphan = read_u32(232);

        let mut hash_seed = [0u32; 4];
        for i in 0..4 {
            hash_seed[i] = read_u32(236 + i * 4);
        }

        let def_hash_version = read_u8(252);
        let jnl_backup_type = read_u8(253);
        let desc_size = read_u16(254);
        let default_mount_opts = read_u32(256);
        let first_meta_bg = read_u32(260);
        let mkfs_time = read_u32(264);

        let mut jnl_blocks = [0u32; 17];
        for i in 0..17 {
            jnl_blocks[i] = read_u32(268 + i * 4);
        }

        let blocks_count_hi = read_u32(336);
        let reserved_blocks_count_hi = read_u32(340);
        let free_blocks_count_hi = read_u32(344);
        let min_extra_isize = read_u16(348);
        let want_extra_isize = read_u16(350);
        let flags = read_u32(352);
        let raid_stride = read_u16(356);
        let mmp_interval = read_u8(358);
        let mmp_block_lo = read_u32(359);
        let raid_stripe_width = read_u32(363);
        let checksum_type = read_u8(367);
        let padding = read_u8(368);
        let checksum_seed = read_u32(369);
        let wtime_hi = read_u16(373);
        let mtime_hi = read_u16(375);
        let mkfs_time_hi = read_u16(377);
        let awtime_hi = read_u16(379);
        let checksum = read_u32(381);

        // Combine high and low parts for 64-bit values
        let blocks_count = ((blocks_count_hi as u64) << 32) | (blocks_count_lo as u64);
        let reserved_blocks_count =
            ((reserved_blocks_count_hi as u64) << 32) | (reserved_blocks_count_lo as u64);
        let free_blocks_count =
            ((free_blocks_count_hi as u64) << 32) | (free_blocks_count_lo as u64);
        let mmp_block = ((mmp_block_lo as u64) << 32) | (mmp_block_lo as u64);

        // Calculate block size
        let block_size = 1024 << log_block_size;
        let cluster_size = if log_cluster_size > 0 {
            1024 << log_cluster_size
        } else {
            block_size
        };

        debug!(
            "Superblock: magic={:#x}, block_size={}, first_data_block={}, inodes_per_group={}",
            magic, block_size, first_data_block, inodes_per_group
        );

        Ok(Self {
            inodes_count,
            blocks_count,
            reserved_blocks_count,
            free_blocks_count,
            free_inodes_count,
            first_data_block,
            block_size,
            cluster_size,
            blocks_per_group,
            clusters_per_group,
            inodes_per_group,
            mount_time,
            write_time,
            mount_count,
            max_mount_count,
            magic,
            state,
            errors,
            minor_rev_level,
            last_check_time,
            check_interval,
            creator_os,
            rev_level,
            default_reserved_uid,
            default_reserved_gid,
            first_inode,
            inode_size,
            block_group_nr,
            feature_compat,
            feature_incompat,
            feature_ro_compat,
            uuid,
            volume_name,
            last_mounted,
            algorithm_usage_bitmap,
            prealloc_blocks,
            prealloc_dir_blocks,
            reserved_gdt_blocks,
            journal_uuid,
            journal_inum,
            journal_dev,
            last_orphan,
            hash_seed,
            def_hash_version,
            jnl_backup_type,
            desc_size,
            default_mount_opts,
            first_meta_bg,
            mkfs_time,
            jnl_blocks,
            blocks_count_hi,
            reserved_blocks_count_hi,
            free_blocks_count_hi,
            min_extra_isize,
            want_extra_isize,
            flags,
            raid_stride,
            mmp_interval,
            mmp_block,
            raid_stripe_width,
            checksum_type,
            padding,
            checksum_seed,
            wtime_hi,
            mtime_hi,
            mkfs_time_hi,
            awtime_hi,
            checksum,
        })
    }

    /// Validate the superblock
    pub fn validate(&self) -> Ext4Result<()> {
        if self.magic != 0xEF53 {
            error!("Invalid ext4 magic number: 0x{:04X}", self.magic);
            return Err(Ext4Error::InvalidMagic);
        }

        if self.state != 1 {
            warn!("Filesystem state is not clean: {}", self.state);
        }

        if self.block_size != 1024 && self.block_size != 2048 && self.block_size != 4096 {
            error!("Invalid block size: {}", self.block_size);
            return Err(Ext4Error::InvalidState);
        }

        Ok(())
    }

    /// Getters
    pub fn inodes_count(&self) -> u32 {
        self.inodes_count
    }
    pub fn blocks_count(&self) -> u64 {
        self.blocks_count
    }
    pub fn reserved_blocks_count(&self) -> u64 {
        self.reserved_blocks_count
    }
    pub fn free_blocks_count(&self) -> u64 {
        self.free_blocks_count
    }
    pub fn free_inodes_count(&self) -> u32 {
        self.free_inodes_count
    }
    pub fn first_data_block(&self) -> u32 {
        self.first_data_block
    }
    pub fn block_size(&self) -> u32 {
        self.block_size
    }
    pub fn cluster_size(&self) -> u32 {
        self.cluster_size
    }
    pub fn blocks_per_group(&self) -> u32 {
        self.blocks_per_group
    }
    pub fn clusters_per_group(&self) -> u32 {
        self.clusters_per_group
    }
    pub fn inodes_per_group(&self) -> u32 {
        self.inodes_per_group
    }
    pub fn mount_time(&self) -> u32 {
        self.mount_time
    }
    pub fn write_time(&self) -> u32 {
        self.write_time
    }
    pub fn mount_count(&self) -> u16 {
        self.mount_count
    }
    pub fn max_mount_count(&self) -> u16 {
        self.max_mount_count
    }
    pub fn magic(&self) -> u16 {
        self.magic
    }
    pub fn state(&self) -> u16 {
        self.state
    }
    pub fn errors(&self) -> u16 {
        self.errors
    }
    pub fn minor_rev_level(&self) -> u16 {
        self.minor_rev_level
    }
    pub fn last_check_time(&self) -> u32 {
        self.last_check_time
    }
    pub fn check_interval(&self) -> u32 {
        self.check_interval
    }
    pub fn creator_os(&self) -> u32 {
        self.creator_os
    }
    pub fn rev_level(&self) -> u32 {
        self.rev_level
    }
    pub fn default_reserved_uid(&self) -> u16 {
        self.default_reserved_uid
    }
    pub fn default_reserved_gid(&self) -> u16 {
        self.default_reserved_gid
    }
    pub fn first_inode(&self) -> u32 {
        self.first_inode
    }
    pub fn inode_size(&self) -> u16 {
        self.inode_size
    }
    pub fn block_group_nr(&self) -> u16 {
        self.block_group_nr
    }
    pub fn feature_compat(&self) -> u32 {
        self.feature_compat
    }
    pub fn feature_incompat(&self) -> u32 {
        self.feature_incompat
    }
    pub fn feature_ro_compat(&self) -> u32 {
        self.feature_ro_compat
    }
    pub fn uuid(&self) -> &[u8; 16] {
        &self.uuid
    }
    pub fn volume_name(&self) -> &[u8; 16] {
        &self.volume_name
    }
    pub fn last_mounted(&self) -> &[u8; 64] {
        &self.last_mounted
    }
    pub fn algorithm_usage_bitmap(&self) -> u32 {
        self.algorithm_usage_bitmap
    }
    pub fn prealloc_blocks(&self) -> u8 {
        self.prealloc_blocks
    }
    pub fn prealloc_dir_blocks(&self) -> u8 {
        self.prealloc_dir_blocks
    }
    pub fn reserved_gdt_blocks(&self) -> u16 {
        self.reserved_gdt_blocks
    }
    pub fn journal_uuid(&self) -> &[u8; 16] {
        &self.journal_uuid
    }
    pub fn journal_inum(&self) -> u32 {
        self.journal_inum
    }
    pub fn journal_dev(&self) -> u32 {
        self.journal_dev
    }
    pub fn last_orphan(&self) -> u32 {
        self.last_orphan
    }
    pub fn hash_seed(&self) -> &[u32; 4] {
        &self.hash_seed
    }
    pub fn def_hash_version(&self) -> u8 {
        self.def_hash_version
    }
    pub fn jnl_backup_type(&self) -> u8 {
        self.jnl_backup_type
    }
    pub fn desc_size(&self) -> u16 {
        self.desc_size
    }
    pub fn default_mount_opts(&self) -> u32 {
        self.default_mount_opts
    }
    pub fn first_meta_bg(&self) -> u32 {
        self.first_meta_bg
    }
    pub fn mkfs_time(&self) -> u32 {
        self.mkfs_time
    }
    pub fn jnl_blocks(&self) -> &[u32; 17] {
        &self.jnl_blocks
    }
    pub fn blocks_count_hi(&self) -> u32 {
        self.blocks_count_hi
    }
    pub fn reserved_blocks_count_hi(&self) -> u32 {
        self.reserved_blocks_count_hi
    }
    pub fn free_blocks_count_hi(&self) -> u32 {
        self.free_blocks_count_hi
    }
    pub fn min_extra_isize(&self) -> u16 {
        self.min_extra_isize
    }
    pub fn want_extra_isize(&self) -> u16 {
        self.want_extra_isize
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn raid_stride(&self) -> u16 {
        self.raid_stride
    }
    pub fn mmp_interval(&self) -> u8 {
        self.mmp_interval
    }
    pub fn mmp_block(&self) -> u64 {
        self.mmp_block
    }
    pub fn raid_stripe_width(&self) -> u32 {
        self.raid_stripe_width
    }
    pub fn checksum_type(&self) -> u8 {
        self.checksum_type
    }
    pub fn padding(&self) -> u8 {
        self.padding
    }
    pub fn checksum_seed(&self) -> u32 {
        self.checksum_seed
    }
    pub fn wtime_hi(&self) -> u16 {
        self.wtime_hi
    }
    pub fn mtime_hi(&self) -> u16 {
        self.mtime_hi
    }
    pub fn mkfs_time_hi(&self) -> u16 {
        self.mkfs_time_hi
    }
    pub fn awtime_hi(&self) -> u16 {
        self.awtime_hi
    }
    pub fn checksum(&self) -> u32 {
        self.checksum
    }
}
