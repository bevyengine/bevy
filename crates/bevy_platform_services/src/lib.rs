//! Miscellaneous operating system services which may not be available on all platforms.
//!
//! Code which calls these APIs should be compilable on all platforms, but should gracefully
//! fail where they are not supported. For example, the methods in  [`dirs`] all return
//! `None` on platforms which have no filesystem.

/// APIs that return the location of standard user directories.
pub mod dirs;
