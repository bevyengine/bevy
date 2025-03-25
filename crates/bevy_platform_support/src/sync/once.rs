//! Provides `Once`, `OnceState`, `OnceLock`

pub use implementation::{Once, OnceLock, OnceState};

#[cfg(feature = "std")]
use std::sync as implementation;

#[cfg(not(feature = "std"))]
mod implementation {
    use core::{
        fmt,
        panic::{RefUnwindSafe, UnwindSafe},
    };

    /// Fallback implementation of `OnceLock` from the standard library.
    pub struct OnceLock<T> {
        inner: spin::Once<T>,
    }

    impl<T> OnceLock<T> {
        /// Creates a new empty cell.
        ///
        /// See the standard library for further details.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                inner: spin::Once::new(),
            }
        }

        /// Gets the reference to the underlying value.
        ///
        /// See the standard library for further details.
        pub fn get(&self) -> Option<&T> {
            self.inner.get()
        }

        /// Gets the mutable reference to the underlying value.
        ///
        /// See the standard library for further details.
        pub fn get_mut(&mut self) -> Option<&mut T> {
            self.inner.get_mut()
        }

        /// Sets the contents of this cell to `value`.
        ///
        /// See the standard library for further details.
        pub fn set(&self, value: T) -> Result<(), T> {
            let mut value = Some(value);

            self.inner.call_once(|| value.take().unwrap());

            match value {
                Some(value) => Err(value),
                None => Ok(()),
            }
        }

        /// Gets the contents of the cell, initializing it with `f` if the cell
        /// was empty.
        ///
        /// See the standard library for further details.
        pub fn get_or_init<F>(&self, f: F) -> &T
        where
            F: FnOnce() -> T,
        {
            self.inner.call_once(f)
        }

        /// Consumes the `OnceLock`, returning the wrapped value. Returns
        /// `None` if the cell was empty.
        ///
        /// See the standard library for further details.
        pub fn into_inner(mut self) -> Option<T> {
            self.take()
        }

        /// Takes the value out of this `OnceLock`, moving it back to an uninitialized state.
        ///
        /// See the standard library for further details.
        pub fn take(&mut self) -> Option<T> {
            if self.inner.is_completed() {
                let mut inner = spin::Once::new();

                core::mem::swap(&mut self.inner, &mut inner);

                inner.try_into_inner()
            } else {
                None
            }
        }
    }

    impl<T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for OnceLock<T> {}
    impl<T: UnwindSafe> UnwindSafe for OnceLock<T> {}

    impl<T> Default for OnceLock<T> {
        fn default() -> OnceLock<T> {
            OnceLock::new()
        }
    }

    impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut d = f.debug_tuple("OnceLock");
            match self.get() {
                Some(v) => d.field(v),
                None => d.field(&format_args!("<uninit>")),
            };
            d.finish()
        }
    }

    impl<T: Clone> Clone for OnceLock<T> {
        fn clone(&self) -> OnceLock<T> {
            let cell = Self::new();
            if let Some(value) = self.get() {
                cell.set(value.clone()).ok().unwrap();
            }
            cell
        }
    }

    impl<T> From<T> for OnceLock<T> {
        fn from(value: T) -> Self {
            let cell = Self::new();
            cell.set(value).map(move |_| cell).ok().unwrap()
        }
    }

    impl<T: PartialEq> PartialEq for OnceLock<T> {
        fn eq(&self, other: &OnceLock<T>) -> bool {
            self.get() == other.get()
        }
    }

    impl<T: Eq> Eq for OnceLock<T> {}

    /// Fallback implementation of `Once` from the standard library.
    pub struct Once {
        inner: OnceLock<()>,
    }

    impl Once {
        /// Creates a new `Once` value.
        ///
        /// See the standard library for further details.
        #[expect(clippy::new_without_default, reason = "matching std::sync::Once")]
        pub const fn new() -> Self {
            Self {
                inner: OnceLock::new(),
            }
        }

        /// Performs an initialization routine once and only once. The given closure
        /// will be executed if this is the first time `call_once` has been called,
        /// and otherwise the routine will *not* be invoked.
        ///
        /// See the standard library for further details.
        pub fn call_once<F: FnOnce()>(&self, f: F) {
            self.inner.get_or_init(f);
        }

        /// Performs the same function as [`call_once()`] except ignores poisoning.
        ///
        /// See the standard library for further details.
        pub fn call_once_force<F: FnOnce(&OnceState)>(&self, f: F) {
            const STATE: OnceState = OnceState { _private: () };

            self.call_once(move || f(&STATE));
        }

        /// Returns `true` if some [`call_once()`] call has completed
        /// successfully. Specifically, `is_completed` will return false in
        /// the following situations:
        ///   * [`call_once()`] was not called at all,
        ///   * [`call_once()`] was called, but has not yet completed,
        ///   * the [`Once`] instance is poisoned
        ///
        /// See the standard library for further details.
        pub fn is_completed(&self) -> bool {
            self.inner.get().is_some()
        }
    }

    impl RefUnwindSafe for Once {}
    impl UnwindSafe for Once {}

    impl fmt::Debug for Once {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Once").finish_non_exhaustive()
        }
    }

    /// Fallback implementation of `OnceState` from the standard library.
    pub struct OnceState {
        _private: (),
    }

    impl OnceState {
        /// Returns `true` if the associated [`Once`] was poisoned prior to the
        /// invocation of the closure passed to [`Once::call_once_force()`].
        ///
        /// See the standard library for further details.
        pub fn is_poisoned(&self) -> bool {
            false
        }
    }

    impl fmt::Debug for OnceState {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("OnceState")
                .field("poisoned", &self.is_poisoned())
                .finish()
        }
    }
}
