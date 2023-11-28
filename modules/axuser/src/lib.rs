//! user information management system used by [ArceOS](https://github.com/rcore-os/arceos).
//!
//! The implementation is based on [`axfs_vfs`].

#![cfg_attr(not(test), no_std)]

pub mod api;
mod sha1;
pub mod user;

extern crate alloc;
