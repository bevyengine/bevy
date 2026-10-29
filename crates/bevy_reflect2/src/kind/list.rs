use crate::{Reflect, TypeRef};
use alloc::boxed::Box;
use bevy_ptr::{Ptr, PtrMut};
use core::any::Any;

/// Describes a grow-able, ordered sequence of items (ex: `Vec<T>`).
///
/// Unsafe code relies on `cast_list_access` being valid for the described type, so this can only be
/// constructed using [`List::new`].
#[derive(Debug)]
pub struct List {
    item: TypeRef,
    cast_list_access: fn(*const u8) -> *const dyn ListAccess,
}

impl List {
    /// Create a new list definition.
    ///
    /// # Safety
    ///
    /// `item` must be the type of every item in the list, and `cast_list_access` must cast pointers
    /// to the list to its [`ListAccess`] implementation (see [`cast_list_access`]).
    pub const unsafe fn new(
        item: TypeRef,
        cast_list_access: fn(*const u8) -> *const dyn ListAccess,
    ) -> Self {
        Self {
            item,
            cast_list_access,
        }
    }

    /// The type of each item in the list.
    #[inline]
    pub const fn item(&self) -> TypeRef {
        self.item
    }

    /// Casts the given `list_ptr` of this type to a [`ListAccess`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `list_ptr` is a valid, initialized value that matches this [`List`].
    pub unsafe fn list_access<'a>(&self, list_ptr: Ptr<'a>) -> &'a dyn ListAccess {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `List`, and `List::new`
        // guarantees that the cast_list_access implementation is valid
        unsafe { &*(self.cast_list_access)(list_ptr.as_ptr()) }
    }

    /// Casts the given `list_ptr` of this type to a mutable [`ListAccess`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `list_ptr` is a valid, initialized value that matches this [`List`].
    pub unsafe fn list_access_mut<'a>(&self, list_ptr: PtrMut<'a>) -> &'a mut dyn ListAccess {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `List`, and `List::new`
        // guarantees that the cast_list_access implementation is valid
        unsafe { &mut *(self.cast_list_access)(list_ptr.as_ptr()).cast_mut() }
    }
}

/// Type-erased access to a list.
///
/// Values passed into the list are type checked: if a value does not match [`List::item`], it is
/// returned in an `Err`.
///
/// # Safety
///
/// Every value returned by [`ListAccess::get`] / [`ListAccess::get_mut`] (and every value removed
/// from the list) must have the type described by [`List::item`].
#[expect(
    clippy::len_without_is_empty,
    reason = "`is_empty` is provided by the `Ref` / `Mut` wrappers"
)]
pub unsafe trait ListAccess: Any {
    /// Returns the number of items in the list.
    fn len(&self) -> usize;
    /// Returns a reference to the value in the list at the `index`, if it exists. Otherwise returns [`None`].
    fn get(&self, index: usize) -> Option<&dyn Reflect>;
    /// Returns a mutable reference to the value in the list at the `index`, if it exists.
    /// Otherwise returns [`None`].
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    /// Inserts `value` at `index`, shifting all items after it.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    fn insert(&mut self, index: usize, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;
    /// Appends `value` to the _back_ of the list. Returns `value` if it does not match [`List::item`].
    fn push(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;
    /// Removes and returns the item at `index`, shifting all items after it. Returns `None` if
    /// `index` is out of bounds.
    fn remove(&mut self, index: usize) -> Option<Box<dyn Reflect>>;
}

/// Casts the given pointer to a [`ListAccess`] pointer.
pub fn cast_list_access<L: ListAccess>(ptr: *const u8) -> *const dyn ListAccess {
    ptr.cast::<L>() as *const dyn ListAccess
}

/// A reflected reference to a [`List`] value, providing read-only reflection operations on that value.
pub struct ListRef<'a> {
    access: &'a dyn ListAccess,
    list_type: &'static List,
}

impl<'a> ListRef<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `list_type`
    pub unsafe fn new(ptr: Ptr<'a>, list_type: &'static List) -> Self {
        Self {
            // SAFETY: the caller guarantees that `ptr` matches `list_type`
            access: unsafe { list_type.list_access(ptr) },
            list_type,
        }
    }

    /// The [`List`] of this type.
    #[inline]
    pub fn list_type(&self) -> &'static List {
        self.list_type
    }

    /// The number of items in the list.
    #[inline]
    pub fn len(&self) -> usize {
        self.access.len()
    }

    /// Returns `true` if there are no items in the list. Otherwise returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a reference to the value in the list at the `index`, if it exists. Otherwise returns [`None`].
    #[inline]
    pub fn get(&self, index: usize) -> Option<&'a dyn Reflect> {
        self.access.get(index)
    }

    /// Returns an iterator over references to all items in the list.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'a dyn Reflect> + use<'a> {
        let access = self.access;
        (0..access.len()).map(move |index| access.get(index).expect("index is less than len"))
    }
}

/// A reflected mutable reference to a [`List`] value, providing reflection operations on that value.
pub struct ListMut<'a> {
    access: &'a mut dyn ListAccess,
    list_type: &'static List,
}

impl<'a> ListMut<'a> {
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `list_type`
    pub unsafe fn new(ptr: PtrMut<'a>, list_type: &'static List) -> Self {
        Self {
            // SAFETY: the caller guarantees that `ptr` matches `list_type`
            access: unsafe { list_type.list_access_mut(ptr) },
            list_type,
        }
    }

    /// Returns a read-only view of this list.
    #[inline]
    pub fn as_ref(&self) -> ListRef<'_> {
        ListRef {
            access: self.access,
            list_type: self.list_type,
        }
    }

    /// The [`List`] of this type.
    #[inline]
    pub fn list_type(&self) -> &'static List {
        self.list_type
    }

    /// The number of items in the list.
    #[inline]
    pub fn len(&self) -> usize {
        self.access.len()
    }

    /// Returns `true` if there are no items in the list. Otherwise returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a reference to the value in the list at the `index`, if it exists. Otherwise returns [`None`].
    #[inline]
    pub fn get(&self, index: usize) -> Option<&dyn Reflect> {
        self.access.get(index)
    }

    /// Returns a mutable reference to the value in the list at the `index`, if it exists. Otherwise returns [`None`].
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.access.get_mut(index)
    }

    /// Returns an iterator over references to all items in the list.
    #[inline]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &dyn Reflect> {
        self.as_ref().iter()
    }

    /// Inserts `value` at `index`, shifting all items after it. Returns `value` if it does not
    /// match [`List::item`].
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    #[inline]
    pub fn insert(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), Box<dyn Reflect>> {
        self.access.insert(index, value)
    }

    /// Appends `value` to the _back_ of the list. Returns `value` if it does not match [`List::item`].
    #[inline]
    pub fn push(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        self.access.push(value)
    }

    /// Removes and returns the item at `index`, shifting all items after it. Returns `None` if
    /// `index` is out of bounds.
    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<Box<dyn Reflect>> {
        self.access.remove(index)
    }

    /// Removes the _back_ element from the list and returns it, or [`None`] if it is empty.
    #[inline]
    pub fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        let last = self.access.len().checked_sub(1)?;
        self.access.remove(last)
    }

    /// Clears all items in the list, resulting in an empty list. Items are removed (and dropped)
    /// back to front, so no items need to be shifted for vec-like lists.
    ///
    /// If this clear is in a "hot" path and you are clearing a large number of items, consider
    /// building a custom clear implementation and adding it to the [`TypeRegistry`](crate::TypeRegistry).
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

/// Implements [`Typed`](crate::Typed) and [`GetTypeRegistration`](crate::GetTypeRegistration) for a
/// list type that implements [`ListAccess`](crate::ListAccess).
///
/// The list must implement [`Default`]. The [`ListAccess`](crate::ListAccess) implementation should
/// call the list's inherent methods using `<$ty>::method` syntax, so the list must have inherent
/// versions of them, otherwise the call resolves to the access trait method itself.
macro_rules! impl_reflect_list {
    (
        impl<$($param:ident),*> for $ty:ty,
        type_path: ($module:literal, $ident:literal),
        item: $item:ty
        $(, where $($bounds:tt)*)?
    ) => {
        $crate::type_path::impl_generic_type_path!(
            impl<$($param),*> for $ty, $module, $ident $(, where $($bounds)*)?
        );

        // SAFETY: `Self` implements `ListAccess` for `$item` items, and the layout and vtable are
        // derived from `Self`
        unsafe impl<$($param),*> $crate::Typed for $ty
        where
            $($param: $crate::Typed,)*
            $($($bounds)*)?
        {
            // SAFETY: `Self` implements `ListAccess` with this item type
            const TYPE: &'static $crate::Type = &unsafe {
                $crate::Type::new(
                    ::core::any::TypeId::of::<Self>(),
                    $crate::Kind::List($crate::List::new(
                        $crate::TypeRef($crate::Type::of::<$item>),
                        $crate::cast_list_access::<Self>,
                    )),
                    Some(::core::alloc::Layout::new::<Self>()),
                    $crate::TypeVTable {
                        set_default: Some($crate::set_default::<Self>),
                        ..$crate::TypeVTable::new::<Self>()
                    },
                    $crate::TypePath::generic::<Self>($module, $ident),
                )
            };
        }

        impl<$($param),*> $crate::GetTypeRegistration for $ty
        where
            $($param: $crate::Typed + $crate::GetTypeRegistration,)*
            $($($bounds)*)?
        {
            fn get_type_registration() -> $crate::TypeRegistration {
                $crate::TypeRegistration::of::<Self>()
            }

            fn register_type_dependencies(registry: &mut $crate::TypeRegistry) {
                $(registry.register::<$param>();)*
            }
        }
    };
}

pub(crate) use impl_reflect_list;
