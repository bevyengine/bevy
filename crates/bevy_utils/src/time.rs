#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

#[cfg(all(not(target_arch = "wasm32"), feature = "std"))]
pub use std::time::Instant;
