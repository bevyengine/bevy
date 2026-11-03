use crate::Reflect;
use alloc::boxed::Box;
use bevy_ptr::Ptr;
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
};

/// Type-erased functions for a [`Type`](crate::Type).
///
/// Every function that takes a [`Ptr`] requires that the pointer is a valid, initialized value of
/// the described type.
///
/// Optional functions are `None` if the type does not support the operation. Operations like
/// [`Clone`], [`Debug`], and [`PartialEq`] fall back to structural implementations driven by
/// [`Kind`](crate::Kind) when they are `None` (see [`Reflect`]).
pub struct TypeVTable {
    /// Casts the pointer to a dyn Reflect pointer using this type's Reflect vtable.
    pub cast_reflect: fn(*const u8) -> *const dyn Reflect,
    /// Sets the given pointer to the "default" value for this type.
    pub set_default: Option<unsafe fn(*mut u8)>,
    /// Returns a clone of the value, or `None` if any part of the value cannot be cloned.
    pub clone: Option<unsafe fn(Ptr<'_>) -> Option<Box<dyn Reflect>>>,
    /// Writes the debug formatting of a pointer to this type, to the given formatter.
    pub debug: Option<unsafe fn(Ptr<'_>, &mut Formatter<'_>) -> fmt::Result>,
    /// Hashes a pointer of this type using the configured hashing logic.
    pub hash: Option<unsafe fn(Ptr<'_>, &mut dyn Hasher)>,
    /// Returns true if the two pointers of the described type are equal.
    pub partial_eq: Option<unsafe fn(Ptr<'_>, Ptr<'_>) -> bool>,
    /// Returns an ordering if the two pointers of the described type have an ordering.
    pub partial_cmp: Option<unsafe fn(Ptr<'_>, Ptr<'_>) -> Option<Ordering>>,
}

impl TypeVTable {
    /// Creates a vtable for `T` with every optional function set to `None`.
    pub const fn new<T: Reflect>() -> Self {
        Self {
            cast_reflect: cast_reflect::<T>,
            set_default: None,
            clone: None,
            debug: None,
            hash: None,
            partial_eq: None,
            partial_cmp: None,
        }
    }
}

/// Casts the pointer to a dyn Reflect pointer using this type's Reflect vtable.
pub fn cast_reflect<T: Reflect>(ptr: *const u8) -> *const dyn Reflect {
    ptr.cast::<T>() as *const dyn Reflect
}

/// Implements [`TypeVTable::clone`] using `T`'s [`Clone`] implementation.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
pub unsafe fn clone<T: Clone + Reflect>(ptr: Ptr<'_>) -> Option<Box<dyn Reflect>> {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    Some(Box::new(unsafe { ptr.deref::<T>() }.clone()))
}

/// Implements [`TypeVTable::clone`] using `func`. Used for custom clone functions.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
#[inline]
pub unsafe fn clone_with<T: Reflect>(
    ptr: Ptr<'_>,
    func: impl FnOnce(&T) -> Option<T>,
) -> Option<Box<dyn Reflect>> {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    let value = func(unsafe { ptr.deref::<T>() })?;
    Some(Box::new(value))
}

/// Implements [`IgnoredFieldVTable::clone`](crate::IgnoredFieldVTable::clone) using `T`'s
/// [`Clone`] implementation.
///
/// # Safety
///
/// `src` must be a valid, initialized `T`, and `dst` must be a valid, unique allocation for a `T`.
/// Any existing value at `dst` will be overwritten without being dropped.
pub unsafe fn clone_into<T: Clone>(src: Ptr<'_>, dst: *mut u8) {
    // SAFETY: the caller guarantees that `src` is a valid `T`, and that `dst` is a valid, unique
    // allocation for a `T`
    unsafe { dst.cast::<T>().write(src.deref::<T>().clone()) }
}

/// Implements [`IgnoredFieldVTable::clone`](crate::IgnoredFieldVTable::clone) using `func`.
///
/// # Safety
///
/// `src` must be a valid, initialized `T`, and `dst` must be a valid, unique allocation for a `T`.
/// Any existing value at `dst` will be overwritten without being dropped.
#[inline]
pub unsafe fn clone_into_with<T>(src: Ptr<'_>, dst: *mut u8, func: impl FnOnce(&T) -> T) {
    // SAFETY: the caller guarantees that `src` is a valid `T`, and that `dst` is a valid, unique
    // allocation for a `T`
    unsafe { dst.cast::<T>().write(func(src.deref::<T>())) }
}

/// Implements [`TypeVTable::debug`] using `T`'s [`Debug`] implementation.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
pub unsafe fn debug<T: Debug>(ptr: Ptr<'_>, f: &mut Formatter<'_>) -> fmt::Result {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    Debug::fmt(unsafe { ptr.deref::<T>() }, f)
}

/// Implements [`TypeVTable::debug`] using `func`.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
#[inline]
pub unsafe fn debug_with<T>(
    ptr: Ptr<'_>,
    f: &mut Formatter<'_>,
    func: impl FnOnce(&T, &mut Formatter<'_>) -> fmt::Result,
) -> fmt::Result {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    func(unsafe { ptr.deref::<T>() }, f)
}

/// Implements [`TypeVTable::hash`] using `T`'s [`Hash`] implementation.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
pub unsafe fn hash<T: Hash>(ptr: Ptr<'_>, mut hasher: &mut dyn Hasher) {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    Hash::hash(unsafe { ptr.deref::<T>() }, &mut hasher);
}

/// Implements [`TypeVTable::hash`] using `func`, which returns the hash of the value.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
#[inline]
pub unsafe fn hash_with<T>(ptr: Ptr<'_>, hasher: &mut dyn Hasher, func: impl FnOnce(&T) -> u64) {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    hasher.write_u64(func(unsafe { ptr.deref::<T>() }));
}

/// Implements [`TypeVTable::partial_eq`] using `T`'s [`PartialEq`] implementation.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
pub unsafe fn partial_eq<T: PartialEq>(a: Ptr<'_>, b: Ptr<'_>) -> bool {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { a.deref::<T>() == b.deref::<T>() }
}

/// Implements [`TypeVTable::partial_eq`] using `func`.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
#[inline]
pub unsafe fn partial_eq_with<T>(
    a: Ptr<'_>,
    b: Ptr<'_>,
    func: impl FnOnce(&T, &T) -> bool,
) -> bool {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { func(a.deref::<T>(), b.deref::<T>()) }
}

/// Implements [`TypeVTable::partial_cmp`] using `T`'s [`PartialOrd`] implementation.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
pub unsafe fn partial_cmp<T: PartialOrd>(a: Ptr<'_>, b: Ptr<'_>) -> Option<Ordering> {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { a.deref::<T>().partial_cmp(b.deref::<T>()) }
}

/// Implements [`TypeVTable::partial_cmp`] using `func`.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
#[inline]
pub unsafe fn partial_cmp_with<T>(
    a: Ptr<'_>,
    b: Ptr<'_>,
    func: impl FnOnce(&T, &T) -> Option<Ordering>,
) -> Option<Ordering> {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { func(a.deref::<T>(), b.deref::<T>()) }
}

/// Sets the given pointer to the "default" value for this type.
///
/// # Safety
///
/// `ptr` must be a valid, unique allocation for a `T`. Any existing value will be overwritten without
/// being dropped.
pub unsafe fn set_default<T: Default>(ptr: *mut u8) {
    // SAFETY: the caller guarantees that `ptr` is a valid, unique allocation for a `T`
    unsafe { set_with(ptr, T::default) }
}

/// Writes the result of `func` to `ptr`. Used to implement [`TypeVTable::set_default`] for custom
/// default functions.
///
/// # Safety
///
/// `ptr` must be a valid, unique allocation for a `T`. Any existing value will be overwritten without
/// being dropped.
#[inline]
pub unsafe fn set_with<T>(ptr: *mut u8, func: impl FnOnce() -> T) {
    // SAFETY: the caller guarantees that `ptr` is a valid, unique allocation for a `T`. `write` does
    // not drop the (uninitialized) previous value.
    unsafe { ptr.cast::<T>().write(func()) }
}
