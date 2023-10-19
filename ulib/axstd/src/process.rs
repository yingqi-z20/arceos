//! A module for working with processes.
//!
//! Since ArceOS is a unikernel, there is no concept of processes. The
//! process-related functions will affect the entire system, such as [`halt`]
//! will shutdown the whole system.

/// Exit and restart the hole system.
pub fn exit(exit_code: i32) {
    arceos_api::sys::ax_restart(exit_code);
}

/// Shutdown the whole system.
pub fn halt(_exit_code: i32) {
    arceos_api::sys::ax_terminate();
}
