use bevy_platform::sync::atomic::AtomicU64;
use bevy_platform::sync::atomic::Ordering;

/// This is used by the [`bsn!`](crate::bsn) macro to generate compile-time only references to symbols. Currently this is used
/// to add IDE support for nested type names, as it allows us to pass the input Ident from the input to the output code.
pub const fn touch_type<T>() {}

/// This is used by the [`bsn!`](crate::bsn) derive to work around [this Rust limitation](https://github.com/rust-lang/rust/issues/86935).
/// A fix is implemented and on track for stabilization. If it is ever implemented, we can remove this.
pub type PathResolveHelper<T> = T;

/// A counter meant to be stored in a `static` to differentiate multiple evaluations of the expression returned by [`bsn!`](crate::bsn)
///
/// Required for named entity references
pub struct CallCounter(AtomicU64);

impl CallCounter {
    /// Creates a new counter starting at 0
    #[expect(
        clippy::new_without_default,
        reason = "Needs to be a associated const fn for use in static"
    )]
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }
    /// Get the current count and then increment this counter by 1
    pub fn increment(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}
