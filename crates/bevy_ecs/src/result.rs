//! Contains error and result helpers for use in fallible systems.

/// A dynamic error type for use in fallible systems.
pub type Error = Box<dyn core::error::Error + Send + Sync + 'static>;

/// A result type for use in fallible systems.
pub type Result<T = (), E = Error> = core::result::Result<T, E>;

/// A convenience constant for returning a successful result in a fallible system.
pub const OK: Result = Ok(());
