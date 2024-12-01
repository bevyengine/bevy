/// A dynamic error type for use in fallible systems.
pub type Error = Box<dyn core::error::Error + Send + Sync + 'static>;

/// A result type for use in fallible systems.
pub type Result<T = (), E = Error> = core::result::Result<T, E>;

/// A convenience function for returning a successful result in a fallible system.
#[inline(always)]
pub const fn ok() -> Result {
    Ok(())
}
