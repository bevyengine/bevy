#[cfg(target_arch = "wasm32")]
pub use web_time::{Duration, Instant, SystemTime, SystemTimeError, TryFromFloatSecsError};

#[cfg(all(not(target_arch = "wasm32"), feature = "std"))]
pub use {
    core::time::{Duration, TryFromFloatSecsError},
    std::time::{Instant, SystemTime, SystemTimeError},
};

#[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
pub use core::time::{Duration, TryFromFloatSecsError};
