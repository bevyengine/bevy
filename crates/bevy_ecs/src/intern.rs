//! Provides types used to statically intern immutable values.
//!
//! Interning is a pattern used to save memory by deduplicating identical values,
//! speed up code by shrinking the stack size of large types,
//! and make comparisons for any type as fast as integers.

use std::{
    fmt::Debug,
    hash::Hash,
    ops::Deref,
    sync::{OnceLock, PoisonError, RwLock},
};

use bevy_utils::HashSet;

/// An interned value. Will stay valid until the end of the program and will not drop.
///
/// For details on interning, see [the module level docs](self).
///
/// # Comparisons
///
/// Interned values use reference equality, meaning they implement [`Eq`]
/// and [`Hash`] regardless of whether `T` implements these traits.
/// Two interned values are only guaranteed to compare equal if they were interned using
/// the same [`Interner`] instance.
// NOTE: This type must NEVER implement Borrow since it does not obey that trait's invariants.
///
/// ```
/// # use bevy_ecs::intern::*;
/// #[derive(PartialEq, Eq, Hash, Debug)]
/// struct Value(i32);
/// impl Internable for Value {
///     // ...
/// # fn leak(&self) -> &'static Self { Box::leak(Box::new(Value(self.0))) }
/// # fn ref_eq(&self, other: &Self) -> bool { std::ptr::eq(self, other ) }
/// # fn ref_hash<H: std::hash::Hasher>(&self, state: &mut H) { std::ptr::hash(self, state); }
/// }
/// let interner_1 = Interner::new();
/// let interner_2 = Interner::new();
/// // Even though both values are identical, their interned forms do not
/// // compare equal as they use different interner instances.
/// assert_ne!(interner_1.intern(&Value(42)), interner_2.intern(&Value(42)));
/// ```
pub struct Interned<T: ?Sized + 'static>(pub &'static T);

impl<T: ?Sized> Deref for Interned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: ?Sized> Clone for Interned<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Interned<T> {}

// Two Interned<T> should only be equal if they are clones from the same instance.
// Therefore, we only use the pointer to determine equality.
impl<T: ?Sized + Internable> PartialEq for Interned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.ref_eq(other.0)
    }
}

impl<T: ?Sized + Internable> Eq for Interned<T> {}

// Important: This must be kept in sync with the PartialEq/Eq implementation
impl<T: ?Sized + Internable> Hash for Interned<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.ref_hash(state);
    }
}

impl<T: ?Sized + Debug> Debug for Interned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<&Interned<T>> for Interned<T> {
    fn from(value: &Interned<T>) -> Self {
        *value
    }
}

/// A trait for internable values.
///
/// This is used by [`Interner<T>`] to create static references for values that are interned.
pub trait Internable: Hash + Eq {
    /// Creates a static reference to `self`, possibly leaking memory.
    fn leak(&self) -> &'static Self;

    /// Returns `true` if the two references point to the same value.
    fn ref_eq(&self, other: &Self) -> bool;

    /// Feeds the reference to the hasher.
    fn ref_hash<H: std::hash::Hasher>(&self, state: &mut H);
}

impl Internable for str {
    fn leak(&self) -> &'static Self {
        let str = self.to_owned().into_boxed_str();
        Box::leak(str)
    }

    fn ref_eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr() && self.len() == other.len()
    }

    fn ref_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        self.as_ptr().hash(state);
    }
}

/// A thread-safe interner which can be used to create [`Interned<T>`] from `&T`
///
/// For details on interning, see [the module level docs](self).
///
/// The implementation ensures that two equal values return two equal [`Interned<T>`] values.
///
/// To use an [`Interner<T>`], `T` must implement [`Internable`].
pub struct Interner<T: ?Sized + 'static>(OnceLock<RwLock<HashSet<&'static T>>>);

impl<T: ?Sized> Interner<T> {
    /// Creates a new empty interner
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }
}

impl<T: Internable + ?Sized> Interner<T> {
    /// Return the [`Interned<T>`] corresponding to `value`.
    ///
    /// If it is called the first time for `value`, it will possibly leak the value and return an
    /// [`Interned<T>`] using the obtained static reference. Subsequent calls for the same `value`
    /// will return [`Interned<T>`] using the same static reference.
    pub fn intern(&self, value: &T) -> Interned<T> {
        let lock = self.0.get_or_init(Default::default);
        {
            let set = lock.read().unwrap_or_else(PoisonError::into_inner);
            if let Some(value) = set.get(value) {
                return Interned(*value);
            }
        }
        {
            let mut set = lock.write().unwrap_or_else(PoisonError::into_inner);
            if let Some(value) = set.get(value) {
                Interned(*value)
            } else {
                let leaked = value.leak();
                set.insert(leaked);
                Interned(leaked)
            }
        }
    }
}

impl<T: ?Sized> Default for Interner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use crate::intern::{Internable, Interned, Interner};

    #[test]
    fn zero_sized_type() {
        #[derive(PartialEq, Eq, Hash, Debug)]
        pub struct A;

        impl Internable for A {
            fn leak(&self) -> &'static Self {
                &A
            }

            fn ref_eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }

            fn ref_hash<H: Hasher>(&self, state: &mut H) {
                std::ptr::hash(self, state);
            }
        }

        let interner = Interner::default();
        let x = interner.intern(&A);
        let y = interner.intern(&A);
        assert_eq!(x, y);
    }

    #[test]
    fn fieldless_enum() {
        #[derive(PartialEq, Eq, Hash, Debug, Clone)]
        pub enum A {
            X,
            Y,
        }

        impl Internable for A {
            fn leak(&self) -> &'static Self {
                match self {
                    A::X => &A::X,
                    A::Y => &A::Y,
                }
            }

            fn ref_eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }

            fn ref_hash<H: Hasher>(&self, state: &mut H) {
                std::ptr::hash(self, state);
            }
        }

        let interner = Interner::default();
        let x = interner.intern(&A::X);
        let y = interner.intern(&A::Y);
        assert_ne!(x, y);
    }

    #[test]
    fn static_sub_strings() {
        let str = "ABC ABC";
        let a = &str[0..3];
        let b = &str[4..7];
        // Same contents
        assert_eq!(a, b);
        let x = Interned(a);
        let y = Interned(b);
        // Different pointers
        assert_ne!(x, y);
        let interner = Interner::default();
        let x = interner.intern(a);
        let y = interner.intern(b);
        // Same pointers returned by interner
        assert_eq!(x, y);
    }

    #[test]
    fn same_interned_instance() {
        let a = Interned("A");
        let b = a;

        assert_eq!(a, b);

        let mut hasher = DefaultHasher::default();
        a.hash(&mut hasher);
        let hash_a = hasher.finish();

        let mut hasher = DefaultHasher::default();
        b.hash(&mut hasher);
        let hash_b = hasher.finish();

        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn same_interned_content() {
        let a = Interned::<str>(Box::leak(Box::new("A".to_string())));
        let b = Interned::<str>(Box::leak(Box::new("A".to_string())));

        assert_ne!(a, b);
    }

    #[test]
    fn different_interned_content() {
        let a = Interned::<str>("A");
        let b = Interned::<str>("B");

        assert_ne!(a, b);
    }
}
