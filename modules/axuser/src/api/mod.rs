//! shell-like high-level user information manipulation operations.

use alloc::string::String;
use axio as io;

/// Verify the password.
pub fn verify(uid: u32, password: String) -> bool {
    crate::user::verify(uid, password)
}

/// Returns the current user name as a [`String`].
pub fn user_name(uid: u32) -> String {
    crate::user::user_name(uid)
}

/// Is the user sudoer.
pub fn is_sudoer(name: String) -> bool {
    crate::user::is_sudoer(name)
}

/// Returns the current user password as a [`String`].
pub fn set_password(password: String) -> io::Result<()> {
    crate::user::set_password(password)
}

/// Returns the name's user id.
pub fn user_id(name: String) -> u32 {
    crate::user::user_id(name)
}
