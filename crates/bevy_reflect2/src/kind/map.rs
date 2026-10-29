use crate::{Reflect, TypeRef};
use alloc::boxed::Box;
use bevy_ptr::{Ptr, PtrMut};

/// Describes a collection of key-value pairs with unique keys (ex: `HashMap<K, V>`).
///
/// Unsafe code relies on `cast_map_access` being valid for the described type, so this can only be
/// constructed using [`Map::new`].
#[derive(Debug)]
pub struct Map {
    key: TypeRef,
    value: TypeRef,
    cast_map_access: fn(*const u8) -> *const dyn MapAccess,
}

impl Map {
    /// Create a new map definition.
    ///
    /// # Safety
    ///
    /// `key` and `value` must be the types of every key and value in the map, and `cast_map_access`
    /// must cast pointers to the map to its [`MapAccess`] implementation (see [`cast_map_access`]).
    pub const unsafe fn new(
        key: TypeRef,
        value: TypeRef,
        cast_map_access: fn(*const u8) -> *const dyn MapAccess,
    ) -> Self {
        Self {
            key,
            value,
            cast_map_access,
        }
    }

    /// The type of each key in the map.
    #[inline]
    pub const fn key(&self) -> TypeRef {
        self.key
    }

    /// The type of each value in the map.
    #[inline]
    pub const fn value(&self) -> TypeRef {
        self.value
    }

    /// Casts the given `map_ptr` of this type to a [`MapAccess`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `map_ptr` is a valid, initialized value that matches this [`Map`].
    pub unsafe fn map_access<'a>(&self, map_ptr: Ptr<'a>) -> &'a dyn MapAccess {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `Map`, and `Map::new`
        // guarantees that the cast_map_access implementation is valid
        unsafe { &*(self.cast_map_access)(map_ptr.as_ptr()) }
    }

    /// Casts the given `map_ptr` of this type to a mutable [`MapAccess`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `map_ptr` is a valid, initialized value that matches this [`Map`].
    pub unsafe fn map_access_mut<'a>(&self, map_ptr: PtrMut<'a>) -> &'a mut dyn MapAccess {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `Map`, and `Map::new`
        // guarantees that the cast_map_access implementation is valid
        unsafe { &mut *(self.cast_map_access)(map_ptr.as_ptr()).cast_mut() }
    }
}

/// A key-value pair that was rejected by [`MapAccess::insert`] because one of them did not match
/// the map's types.
pub type RejectedEntry = (Box<dyn Reflect>, Box<dyn Reflect>);

/// Type-erased access to a map.
///
/// Keys passed in for lookup must have the exact type of [`Map::key`], otherwise the lookup fails.
///
/// # Safety
///
/// Every key and value returned by this trait (and every value removed from the map) must have the
/// types described by [`Map::key`] and [`Map::value`].
#[expect(
    clippy::len_without_is_empty,
    reason = "`is_empty` is provided by the `Ref` / `Mut` wrappers"
)]
pub unsafe trait MapAccess: 'static {
    /// Returns the number of items in the map.
    fn len(&self) -> usize;
    /// Returns a reference to the value in the map whose key is equal to `key` (if it exists).
    fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect>;
    /// Returns a mutable reference to the value in the map whose key is equal to `key` (if it exists).
    fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect>;
    /// Returns an iterator over references to the key-value pairs in the map.
    fn iter(&self) -> Box<dyn Iterator<Item = (&dyn Reflect, &dyn Reflect)> + '_>;
    /// Returns an iterator over references to the key-value pairs in the map, with mutable access to the values.
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (&dyn Reflect, &mut dyn Reflect)> + '_>;
    /// Inserts a key-value pair, returning the previous value for the key (if it exists). If the key
    /// or value does not match the map's types, both are returned in an `Err`.
    fn insert(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Result<Option<Box<dyn Reflect>>, RejectedEntry>;
    /// Removes `key` from the map, returning its value (if it exists).
    fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>>;
    /// Clears all items in the map, resulting in an empty map.
    fn clear(&mut self);
    /// Retains only the entries for which `f` returns `true`.
    fn retain(&mut self, f: &mut dyn FnMut(&dyn Reflect, &mut dyn Reflect) -> bool);
}

/// Casts the given pointer to a [`MapAccess`] pointer.
pub fn cast_map_access<M: MapAccess>(ptr: *const u8) -> *const dyn MapAccess {
    ptr.cast::<M>() as *const dyn MapAccess
}

/// A reflected reference to a [`Map`] value, providing read-only reflection operations on that value.
pub struct MapRef<'a> {
    access: &'a dyn MapAccess,
    map_type: &'static Map,
}

impl<'a> MapRef<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `map_type`
    pub unsafe fn new(ptr: Ptr<'a>, map_type: &'static Map) -> Self {
        Self {
            // SAFETY: the caller guarantees that `ptr` matches `map_type`
            access: unsafe { map_type.map_access(ptr) },
            map_type,
        }
    }

    /// The [`Map`] of this type.
    #[inline]
    pub fn map_type(&self) -> &'static Map {
        self.map_type
    }

    /// The number of items in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.access.len()
    }

    /// Returns `true` if there are no items in the map. Otherwise returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a reference to the value in the map whose key is equal to `key` (if it exists).
    #[inline]
    pub fn get(&self, key: &dyn Reflect) -> Option<&'a dyn Reflect> {
        self.access.get(key)
    }

    /// Returns `true` if the map contains an item whose key is equal to `value`. Otherwise returns `false`.
    #[inline]
    pub fn contains_key(&self, key: &dyn Reflect) -> bool {
        self.get(key).is_some()
    }

    /// Returns an iterator over references to the key-value pairs in the map.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&'a dyn Reflect, &'a dyn Reflect)> + use<'a> {
        self.access.iter()
    }
}

/// A reflected mutable reference to a [`Map`] value, providing reflection operations on that value.
pub struct MapMut<'a> {
    access: &'a mut dyn MapAccess,
    map_type: &'static Map,
}

impl<'a> MapMut<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `map_type`
    pub unsafe fn new(ptr: PtrMut<'a>, map_type: &'static Map) -> Self {
        Self {
            // SAFETY: the caller guarantees that `ptr` matches `map_type`
            access: unsafe { map_type.map_access_mut(ptr) },
            map_type,
        }
    }

    /// Returns a read-only view of this map.
    #[inline]
    pub fn as_ref(&self) -> MapRef<'_> {
        MapRef {
            access: self.access,
            map_type: self.map_type,
        }
    }

    /// The [`Map`] of this type.
    #[inline]
    pub fn map_type(&self) -> &'static Map {
        self.map_type
    }

    /// The number of items in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.access.len()
    }

    /// Returns `true` if there are no items in the map. Otherwise returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a reference to the value in the map whose key is equal to `key` (if it exists).
    #[inline]
    pub fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        self.access.get(key)
    }

    /// Returns a mutable reference to the value in the map whose key is equal to `key` (if it exists).
    #[inline]
    pub fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        self.access.get_mut(key)
    }

    /// Returns `true` if the map contains an item whose key is equal to `value`. Otherwise returns `false`.
    #[inline]
    pub fn contains_key(&self, key: &dyn Reflect) -> bool {
        self.get(key).is_some()
    }

    /// Returns an iterator over references to the key-value pairs in the map.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&dyn Reflect, &dyn Reflect)> {
        self.access.iter()
    }

    /// Returns an iterator over references to the key-value pairs in the map, with mutable access to the values.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&dyn Reflect, &mut dyn Reflect)> {
        self.access.iter_mut()
    }

    /// Inserts a key-value pair, returning the previous value for the key (if it exists). If the key
    /// or value does not match the map's types, both are returned in an `Err`.
    #[inline]
    pub fn insert(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Result<Option<Box<dyn Reflect>>, RejectedEntry> {
        self.access.insert(key, value)
    }

    /// Removes `key` from the map, returning its value (if it exists).
    #[inline]
    pub fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>> {
        self.access.remove(key)
    }

    /// Clears all items in the map, resulting in an empty map.
    #[inline]
    pub fn clear(&mut self) {
        self.access.clear();
    }

    /// Retains only the entries for which `f` returns `true`.
    #[inline]
    pub fn retain(&mut self, mut f: impl FnMut(&dyn Reflect, &mut dyn Reflect) -> bool) {
        self.access.retain(&mut f);
    }
}
