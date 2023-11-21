//! Inspection and manipulation of the process’s environment.

#[cfg(feature = "fs")]
extern crate alloc;

#[cfg(feature = "fs")]
use {crate::io, alloc::string::String};

/// Returns the current working directory as a [`String`].
#[cfg(feature = "fs")]
pub fn current_dir() -> io::Result<String> {
    arceos_api::fs::ax_current_dir()
}

/// Changes the current working directory to the specified path.
#[cfg(feature = "fs")]
pub fn set_current_dir(path: &str) -> io::Result<()> {
    arceos_api::fs::ax_set_current_dir(path)
}

/// Returns the current user id as a [`u32`].
#[cfg(feature = "fs")]
pub fn current_uid() -> io::Result<u32> {
    arceos_api::fs::ax_getuid()
}

/// Changes the current user id to the specified id.
#[cfg(feature = "fs")]
pub fn set_current_uid(uid: u32) -> io::Result<()> {
    arceos_api::fs::ax_setuid(uid)
}

/// Changes the current user id to 0 if the current user is a sudoer.
#[cfg(feature = "fs")]
pub fn sudo() -> io::Result<()> {
    arceos_api::fs::sudo()
}

/// Changes the current user password.
#[cfg(feature = "fs")]
pub fn set_password() -> io::Result<()> {
    arceos_api::fs::ax_setpassword()
}
