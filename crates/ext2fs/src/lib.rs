#![no_std]
#![feature(allocator_api)]
#![feature(new_uninit)]
extern crate alloc;
mod bitmap;
mod block_cache_manager;
mod block_dev;
mod config;
mod efs;
mod inode_manager;
mod layout;
mod mutex;
mod timer;
mod vfs;

use bitmap::Bitmap;
pub use block_dev::BlockDevice;
pub use config::{BLOCKS_PER_GRP, BLOCK_SIZE};
pub use efs::Ext2FileSystem;
use layout::{BlockGroupDesc, DiskInode, SuperBlock};
pub use layout::{EXT2_S_IFDIR, EXT2_S_IFREG};
pub use timer::{TimeProvider, ZeroTimeProvider};
pub use vfs::Inode;
use vfs::InodeCache;
