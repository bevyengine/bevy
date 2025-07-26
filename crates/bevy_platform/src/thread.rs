//! Provides `sleep` for all platforms.

pub use thread::sleep;

crate::cfg::switch! {
    // TODO: use browser timeouts based on ScheduleRunnerPlugin::build
    // crate::cfg::web => { ... }
    crate::cfg::std => {
        use std::thread;
    }
    _ => {
        mod fallback {
            use core::{hint::spin_loop, time::Duration};

            use crate::time::Instant;

            /// Puts the current thread to sleep for at least the specified amount of time.
            ///
            /// As this is a `no_std` fallback implementation, this will spin the current thread.
            pub fn sleep(dur: Duration) {
                let start = Instant::now();

                while start.elapsed() < dur {
                    spin_loop();
                }
            }
        }

        use fallback as thread;
    }
}
