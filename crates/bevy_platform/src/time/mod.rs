//! Provides `Instant` for all platforms.

pub use time::Instant;

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        use web_time as time;
    } else if #[cfg(feature = "std")] {
        use std::time;
    } else {
        mod fallback;

        use fallback as time;
    }
}
