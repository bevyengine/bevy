use crate::{Reflect, TypeRef};
use alloc::boxed::Box;
use bevy_ptr::{Ptr, PtrMut};
use core::any::Any;

/// Describes a collection of unique values (ex: `HashSet<T>`).
///
/// Unsafe code relies on `cast_set_access` being valid for the described type, so this can only be
/// constructed using [`Set::new`].
#[derive(Debug)]
pub struct Set {
    value: TypeRef,
    cast_set_access: fn(*const u8) -> *const dyn SetAccess,
}

impl Set {
    /// Create a new set definition.
    ///
    /// # Safety
    ///
    /// `value` must be the type of every value in the set, and `cast_set_access` must cast pointers
    /// to the set to its [`SetAccess`] implementation (see [`cast_set_access`]).
    pub const unsafe fn new(
        value: TypeRef,
        cast_set_access: fn(*const u8) -> *const dyn SetAccess,
    ) -> Self {
        Self {
            value,
            cast_set_access,
        }
    }

    /// The type of each value in the set.
    #[inline]
    pub const fn value(&self) -> TypeRef {
        self.value
    }

    /// Casts the given `set_ptr` of this type to a [`SetAccess`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `set_ptr` is a valid, initialized value that matches this [`Set`].
    pub unsafe fn set_access<'a>(&self, set_ptr: Ptr<'a>) -> &'a dyn SetAccess {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `Set`, and `Set::new`
        // guarantees that the cast_set_access implementation is valid
        unsafe { &*(self.cast_set_access)(set_ptr.as_ptr()) }
    }

    /// Casts the given `set_ptr` of this type to a mutable [`SetAccess`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `set_ptr` is a valid, initialized value that matches this [`Set`].
    pub unsafe fn set_access_mut<'a>(&self, set_ptr: PtrMut<'a>) -> &'a mut dyn SetAccess {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `Set`, and `Set::new`
        // guarantees that the cast_set_access implementation is valid
        unsafe { &mut *(self.cast_set_access)(set_ptr.as_ptr()).cast_mut() }
    }
}

/// Type-erased access to a set.
///
/// Values passed in for lookup must have the exact type of [`Set::value`], otherwise the lookup fails.
///
/// # Safety
///
/// Every value returned by this trait must have the type described by [`Set::value`].
#[expect(
    clippy::len_without_is_empty,
    reason = "`is_empty` is provided by the `Ref` / `Mut` wrappers"
)]
pub unsafe trait SetAccess: Any {
    /// Returns the number of items in the set.
    fn len(&self) -> usize;
    /// Returns a reference to the value in the set that is equal to `value` (if it exists).
    fn get(&self, value: &dyn Reflect) -> Option<&dyn Reflect>;
    /// Returns an iterator over the values in this set.
    fn iter(&self) -> Box<dyn Iterator<Item = &dyn Reflect> + '_>;
    /// Inserts `value`, returning `true` if it was not already present. If `value` does not match
    /// the set's type, it is returned in an `Err`.
    fn insert(&mut self, value: Box<dyn Reflect>) -> Result<bool, Box<dyn Reflect>>;
    /// Removes `value` from the set, returning `true` if it was present.
    fn remove(&mut self, value: &dyn Reflect) -> bool;
    /// Clears all values in the set, resulting in an empty set.
    fn clear(&mut self);
    /// Retains only the values for which `f` returns `true`.
    fn retain(&mut self, f: &mut dyn FnMut(&dyn Reflect) -> bool);
}

/// Casts the given pointer to a [`SetAccess`] pointer.
pub fn cast_set_access<S: SetAccess>(ptr: *const u8) -> *const dyn SetAccess {
    ptr.cast::<S>() as *const dyn SetAccess
}

/// A reflected reference to a [`Set`] value, providing read-only reflection operations on that value.
pub struct SetRef<'a> {
    access: &'a dyn SetAccess,
    set_type: &'static Set,
}

impl<'a> SetRef<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `set_type`
    pub unsafe fn new(ptr: Ptr<'a>, set_type: &'static Set) -> Self {
        Self {
            // SAFETY: the caller guarantees that `ptr` matches `set_type`
            access: unsafe { set_type.set_access(ptr) },
            set_type,
        }
    }

    /// The [`Set`] of this type.
    #[inline]
    pub fn set_type(&self) -> &'static Set {
        self.set_type
    }

    /// The number of items in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.access.len()
    }

    /// Returns `true` if there are no items in the set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the item in the set equal to `value`, if it is present in the set. Otherwise returns [`None`].
    #[inline]
    pub fn get(&self, value: &dyn Reflect) -> Option<&'a dyn Reflect> {
        self.access.get(value)
    }

    /// Returns `true` if the set contains an item equal to `value`. Otherwise returns `false`.
    #[inline]
    pub fn contains(&self, value: &dyn Reflect) -> bool {
        self.get(value).is_some()
    }

    /// Returns an iterator over the items in the set.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'a dyn Reflect> + use<'a> {
        self.access.iter()
    }
}

/// A reflected mutable reference to a [`Set`] value, providing reflection operations on that value.
pub struct SetMut<'a> {
    access: &'a mut dyn SetAccess,
    set_type: &'static Set,
}

impl<'a> SetMut<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `set_type`
    pub unsafe fn new(ptr: PtrMut<'a>, set_type: &'static Set) -> Self {
        Self {
            // SAFETY: the caller guarantees that `ptr` matches `set_type`
            access: unsafe { set_type.set_access_mut(ptr) },
            set_type,
        }
    }

    /// Returns a read-only view of this set.
    #[inline]
    pub fn as_ref(&self) -> SetRef<'_> {
        SetRef {
            access: self.access,
            set_type: self.set_type,
        }
    }

    /// The [`Set`] of this type.
    #[inline]
    pub fn set_type(&self) -> &'static Set {
        self.set_type
    }

    /// The number of items in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.access.len()
    }

    /// Returns `true` if there are no items in the set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the item in the set equal to `value`, if it is present in the set. Otherwise returns [`None`].
    #[inline]
    pub fn get(&self, value: &dyn Reflect) -> Option<&dyn Reflect> {
        self.access.get(value)
    }

    /// Returns `true` if the set contains an item equal to `value`. Otherwise returns `false`.
    #[inline]
    pub fn contains(&self, value: &dyn Reflect) -> bool {
        self.get(value).is_some()
    }

    /// Returns an iterator over the items in the set.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &dyn Reflect> {
        self.access.iter()
    }

    /// Inserts `value`, returning `true` if it was not already present. If `value` does not match
    /// the set's type, it is returned in an `Err`.
    #[inline]
    pub fn insert(&mut self, value: Box<dyn Reflect>) -> Result<bool, Box<dyn Reflect>> {
        self.access.insert(value)
    }

    /// Removes `value` from the set, returning `true` if it was present.
    #[inline]
    pub fn remove(&mut self, value: &dyn Reflect) -> bool {
        self.access.remove(value)
    }

    /// Clears all values in the set, resulting in an empty set.
    #[inline]
    pub fn clear(&mut self) {
        self.access.clear();
    }

    /// Retains only the values for which `f` returns `true`.
    #[inline]
    pub fn retain(&mut self, mut f: impl FnMut(&dyn Reflect) -> bool) {
        self.access.retain(&mut f);
    }
}
