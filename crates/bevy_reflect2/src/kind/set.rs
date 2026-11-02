use crate::{Reflect, TypeRef};
use alloc::boxed::Box;
use bevy_ptr::{Ptr, PtrMut};

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
    reason = "`is_empty` is provided by `impl dyn SetAccess`"
)]
pub unsafe trait SetAccess: 'static {
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

impl dyn SetAccess {
    /// Returns `true` if there are no items in the set. Otherwise returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the set contains an item equal to `value`. Otherwise returns `false`.
    #[inline]
    pub fn contains(&self, value: &dyn Reflect) -> bool {
        self.get(value).is_some()
    }
}
