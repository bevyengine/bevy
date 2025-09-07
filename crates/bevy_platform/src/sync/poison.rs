//! Provides `TryLockError`, `TryLockResult`

use core::{error::Error, fmt};

/// Fallback implementation of `TryLockError` from the standard library.
pub enum TryLockError {
    /// The lock could not be acquired at this time because the operation would
    /// otherwise block.
    WouldBlock,
}

impl fmt::Debug for TryLockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::WouldBlock => "WouldBlock".fmt(f),
        }
    }
}

impl fmt::Display for TryLockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::WouldBlock => "try_lock failed because the operation would block",
        }
        .fmt(f)
    }
}

impl Error for TryLockError {}

/// Fallback implementation of `TryLockResult` from the standard library.
pub type TryLockResult<Guard> = Result<Guard, TryLockError>;
