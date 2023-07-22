//! Provides types used to statically intern immutable values.
//!
//! Interning is a pattern used to save memory by deduplicating identical values,
//! speed up code by shrinking the stack size of large types,
//! and make comparisons for any type as fast as integers.

use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    hash::Hash,
    ops::Deref,
    ptr,
    sync::{OnceLock, PoisonError, RwLock},
};

use crate::HashSet;

/// An interned value. Will stay valid until the end of the program and will not drop.
///
/// For details on interning, see [the module level docs](self).
///
/// # Implementation details
///
/// This is just a reference to a value with an `'static` lifetime.
///
/// This implements [`Copy`], [`Clone`], [`PartialEq`], [`Eq`] and [`Hash`] regardles of what `T`
/// implements, because it only uses the pointer to the value and not the value itself.
/// Therefore it MUST NEVER implement [`Borrow`](`std::borrow::Borrow`).
pub struct Interned<T: ?Sized + 'static>(&'static T);

impl<T: ?Sized> Deref for Interned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: ?Sized> Clone for Interned<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T: ?Sized> Copy for Interned<T> {}

// Two Interned<T> should only be equal if they are clones from the same instance.
// Therefore, we only use the pointer to determine equality.
impl<T: ?Sized> PartialEq for Interned<T> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.0, other.0)
    }
}

impl<T: ?Sized> Eq for Interned<T> {}

// Important: This must be kept in sync with the PartialEq/Eq implementation
impl<T: ?Sized> Hash for Interned<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ptr::hash(self.0, state);
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

/// A trait for leaking data.
///
/// This is used by [`Interner<T>`] to create static references for values that are interned.
///
/// It is implemented for static references of `T`, [`Box<T>`], and [`Cow<'static, T>`].
pub trait Leak<T: ?Sized>: Borrow<T> {
    /// Creates a static reference to a value equal to `self.borrow()`, possibly leaking memory.
    fn leak(self) -> &'static T;
}

impl<T: ?Sized> Leak<T> for &'static T {
    fn leak(self) -> &'static T {
        self
    }
}

impl<T: ?Sized> Leak<T> for Box<T> {
    fn leak(self) -> &'static T {
        Box::leak(self)
    }
}

impl<T: Clone + ?Sized> Leak<T> for Cow<'static, T> {
    fn leak(self) -> &'static T {
        match self {
            Cow::Borrowed(value) => value,
            Cow::Owned(value) => Box::leak(Box::new(value)),
        }
    }
}

/// A thread-safe interner which can be used to create [`Interned<T>`] from `&T`
///
/// For details on interning, see [the module level docs](self).
///
/// The implementation ensures that two equal values return two equal [`Interned<T>`] values.
pub struct Interner<T: ?Sized + 'static>(OnceLock<RwLock<HashSet<&'static T>>>);

impl<T: ?Sized> Interner<T> {
    /// Creates a new empty interner
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }
}

impl<T: Hash + Eq + ?Sized> Interner<T> {
    /// Return the [`Interned<T>`] corresponding to `value`.
    ///
    /// If it is called the first time for `value`, it will possibly leak the value and return an
    /// [`Interned<T>`] using the obtained static reference. Subsequent calls for the same `value`
    /// will return [`Interned<T>`] using the same static reference.
    pub fn intern<Q: Leak<T>>(&self, value: Q) -> Interned<T> {
        let lock = self.0.get_or_init(Default::default);
        {
            let set = lock.read().unwrap_or_else(PoisonError::into_inner);
            if let Some(value) = set.get(value.borrow()) {
                return Interned(*value);
            }
        }
        {
            let mut set = lock.write().unwrap_or_else(PoisonError::into_inner);
            if let Some(value) = set.get(value.borrow()) {
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

/// A type that can provide static references to equal values.
pub trait StaticRef {
    /// Returns a static reference to a value equal to `self`, if possible.
    /// This method is used by [`OptimizedInterner::intern`] to optimize the interning process.
    ///
    /// # Invariant
    ///
    /// The following invariants must be hold:
    ///
    /// `ptr_eq(a.static_ref(), b.static_ref())` if `a == b`
    /// `ptr_neq(a.static_ref(), b.static_ref())` if `a != b`
    ///
    /// where `ptr_eq` and `ptr_neq` are defined as :
    /// ```
    /// fn ptr_eq<T>(x: Option<&'static T>, y: Option<&'static T>) -> bool {
    ///     match (x, y) {
    ///         (Some(x), Some(y)) => std::ptr::eq(x, y),
    ///         (None, None) => true,
    ///         _ => false,
    ///     }
    /// }
    ///
    /// fn ptr_neq<T>(x: Option<&'static T>, y: Option<&'static T>) -> bool {
    ///     match (x, y) {
    ///         (Some(x), Some(y)) => !std::ptr::eq(x, y),
    ///         (None, None) => true,
    ///         _ => false,
    ///     }
    /// }
    /// ```
    fn static_ref(&self) -> Option<&'static Self>;
}

/// An optimized [`Interner`] for  types that implement [`StaticRef`].
pub struct OptimizedInterner<T: ?Sized + 'static>(Interner<T>);

impl<T: ?Sized> OptimizedInterner<T> {
    /// Creates a new empty interner
    pub const fn new() -> Self {
        Self(Interner::new())
    }
}

impl<T: StaticRef + Hash + Eq + ?Sized> OptimizedInterner<T> {
    /// Return the [`Interned<T>`] corresponding to `value`.
    ///
    /// If it is called the first time for `value`, it will possibly leak the value and return an
    /// [`Interned<T>`] using the obtained static reference. Subsequent calls for the same `value`
    /// will return [`Interned<T>`] using the same static reference.
    ///
    /// Compared to [`Interner::intern`], this uses [`StaticRef::static_ref`] to short-circuit
    /// the interning process.
    pub fn intern<Q: Leak<T>>(&self, value: Q) -> Interned<T> {
        if let Some(value) = value.borrow().static_ref() {
            return Interned(value);
        }
        self.0.intern(value)
    }
}

impl<T: ?Sized> Default for OptimizedInterner<T> {
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

    use crate::intern::{Interned, Interner, Leak};

    #[test]
    fn zero_sized_type() {
        #[derive(PartialEq, Eq, Hash, Debug)]
        pub struct A;

        impl Leak<A> for A {
            fn leak(self) -> &'static Self {
                &A
            }
        }

        let interner = Interner::default();
        let x = interner.intern(&A);
        let y = interner.intern(&A);
        assert_eq!(x, y);
    }

    #[test]
    fn fieldless_enum() {
        #[derive(PartialEq, Eq, Hash, Debug)]
        pub enum A {
            X,
            Y,
        }

        impl Leak<A> for A {
            fn leak(self) -> &'static Self {
                match self {
                    A::X => &A::X,
                    A::Y => &A::Y,
                }
            }
        }

        let interner = Interner::default();
        let x = interner.intern(A::X);
        let y = interner.intern(A::Y);
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
        let a = Interned(Box::leak(Box::new("A")));
        let b = Interned(Box::leak(Box::new("A")));

        assert_ne!(a, b);
    }

    #[test]
    fn different_interned_content() {
        let a = Interned(Box::leak(Box::new("A")));
        let b = Interned(Box::leak(Box::new("B")));

        assert_ne!(a, b);
    }
}
