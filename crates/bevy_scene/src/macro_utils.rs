use std::sync::LazyLock;

use bevy_platform::sync::atomic::AtomicU64;
use bevy_platform::sync::atomic::Ordering;

/// This is used by the [`bsn!`](crate::bsn) macro to generate compile-time only references to symbols. Currently this is used
/// to add IDE support for nested type names, as it allows us to pass the input Ident from the input to the output code.
pub const fn touch_type<T>() {}

/// This is used by the [`bsn!`](crate::bsn) derive to work around [this Rust limitation](https://github.com/rust-lang/rust/issues/86935).
/// A fix is implemented and on track for stabilization. If it is ever implemented, we can remove this.
pub type PathResolveHelper<T> = T;

/// A counter meant to uniquely identify the scopes of [`bsn!`](crate::bsn) and
/// [`bsn_list!`](crate::bsn_list).
///
/// Required for named entity references.
pub struct InvocationCounter(AtomicU64);

impl InvocationCounter {
    /// Creates a new counter starting at 0
    #[expect(
        clippy::new_without_default,
        reason = "Needs to be an associated const fn for use in static"
    )]
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }
    /// Get the current count and then increment this counter by 1.
    pub fn increment(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

/// A global counter used to assign each [`bsn!`](crate::bsn) and [`bsn_list!`](crate::bsn_list)
/// macro invocation site a unique invocation ID.
pub static INVOCATION_COUNTER: InvocationCounter = InvocationCounter::new();

/// Create a [`LazyLock`] that initializes with an ID from the shared [`INVOCATION_COUNTER`].
pub const fn get_lazy_invocation_id() -> LazyLock<u64> {
    fn init() -> u64 {
        INVOCATION_COUNTER.increment()
    }
    LazyLock::new(init)
}

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
