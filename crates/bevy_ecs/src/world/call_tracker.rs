//! Call location tracker which can be stored and later recalled.

/// With feature `command_tracking` this can be used to display the original call site.
/// When the feature is disabled `CallTracker` is zero-sized, and `CallTracker::track()` will emit the unit type to avoid any overhead.
#[derive(Debug)]
pub struct CallTracker {
    #[cfg(feature = "command_tracking")]
    caller: String,
}

#[allow(clippy::derivable_impls)]
impl Default for CallTracker {
    #[cfg_attr(feature = "command_tracking", track_caller)]
    #[cfg_attr(not(feature = "command_tracking"), inline(always))]
    fn default() -> Self {
        Self {
            #[cfg(feature = "command_tracking")]
            caller: format!(
                "(Command origin: \"{}\")",
                std::panic::Location::caller()
            ),
        }
    }
}

impl CallTracker {
    // /// Construct a new call tracker with the calling location.
    // /// This may be called from a function with the [`track_caller`] attribute
    // /// to use the parent caller's location.
    // pub fn new() -> Self {
    //     Self {
    //         #[cfg(feature = "command_tracking")]
    //         caller: format!(
    //             "(Command origin: \"{}\")",
    //             std::panic::Location::caller().to_string()
    //         ),
    //     }
    // }

    /// Returns the tracked location when the `command_tracking` feature is enabled,
    /// and unit `()` otherwise.
    #[cfg(feature = "command_tracking")]
    pub fn track(&self) -> CallTrackRef {
        self.caller.as_ref()
    }

    /// Returns the tracked location when the `command_tracking` feature is enabled,
    /// and unit `()` otherwise.
    #[cfg(not(feature = "command_tracking"))]
    #[inline(always)]
    pub fn track(&self) -> CallTrackRef {
        CallTrackRef
    }
}

/// Return type of the CallTracker::track() method
#[cfg(feature = "command_tracking")]
pub type CallTrackRef<'a> = &'a str;
/// Return type of the CallTracker::track() method
#[cfg(not(feature = "command_tracking"))]
#[derive(Clone, Copy)]
pub struct CallTrackRef;

/// Display the caller, or a message describing the feature if not enabled
#[cfg(feature = "command_tracking")]
#[macro_export]
macro_rules! show_tracked_caller {
    ($caller:expr) => {
        $caller
    };
}

/// Display the caller, or a message describing the feature if not enabled
#[cfg(not(feature = "command_tracking"))]
#[macro_export]
macro_rules! show_tracked_caller {
    ($caller:expr) => {{
        let _ = $caller;
        "(Run with `features=\"bevy/command_tracking\"` to show the command origin)"
    }};
}

pub use show_tracked_caller;
