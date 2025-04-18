use bevy_platform::sync::atomic::{AtomicBool, Ordering};

/// Wrapper around an [`AtomicBool`], abstracting the backing implementation and
/// ordering considerations.
#[doc(hidden)]
pub struct OnceFlag(AtomicBool);

impl OnceFlag {
    /// Create a new flag in the unset state.
    pub const fn new() -> Self {
        Self(AtomicBool::new(true))
    }

    /// Sets this flag. Will return `true` if this flag hasn't been set before.
    pub fn set(&self) -> bool {
        self.0.swap(false, Ordering::Relaxed)
    }
}

impl Default for OnceFlag {
    fn default() -> Self {
        Self::new()
    }
}

/// Call some expression only once per call site.
#[macro_export]
macro_rules! once {
    ($expression:expr) => {{
        static SHOULD_FIRE: $crate::OnceFlag = $crate::OnceFlag::new();
        if SHOULD_FIRE.set() {
            $expression;
        }
    }};
}
