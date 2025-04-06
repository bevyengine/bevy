//! Provides [`exit`] and [`abort`] and all platforms.
//! These are methods of absolute last resort and are almost _never_ the right choice.
//! Consider using [`Result`] or [`panic!`] instead.

pub use process::{abort, exit};

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::process;
    } else {
        mod fallback {
            /// Terminates the process in an abnormal fashion.
            #[cold]
            pub fn abort() -> ! {
                // For no_std targets, panicking while panicking is defined as an abort
                struct Bomb;

                impl Drop for Bomb {
                    fn drop(&mut self) {
                        panic!("Panicking while panicking to abort")
                    }
                }

                let _bomb = Bomb;
                panic!("Panicking while panicking to abort")
            }

            /// Terminates the current process with the specified exit code.
            #[cold]
            pub fn exit(_code: i32) -> ! {
                abort();
            }
        }

        use fallback as process;
    }
}
