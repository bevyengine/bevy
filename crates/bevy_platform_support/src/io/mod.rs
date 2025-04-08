//! Provides standard types for IO systems.

pub use io::{Error, ErrorKind, Result};

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::io as io;
    } else {
        mod fallback;

        use fallback as io;
    }
}
