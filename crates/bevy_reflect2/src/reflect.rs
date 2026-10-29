pub use bevy_reflect2_macros::Reflect;

use crate::{
    kind::Kind,
    ty::{Type, Typed},
    ArrayMut, ArrayRef, EnumMut, EnumRef, KindMut, KindRef, ListMut, ListRef, MapMut, MapRef,
    SetMut, SetRef, StructMut, StructRef,
};
use alloc::boxed::Box;
use bevy_ptr::{Ptr, PtrMut};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    fmt::{self, Debug, Formatter},
    ops::{Deref, DerefMut},
    ptr,
};

/// A reflected value. This is implemented for every [`Typed`] type.
///
/// # Core traits
///
/// `dyn Reflect` provides reflected versions of [`Clone`], [`Debug`](core::fmt::Debug),
/// [`Hash`](core::hash::Hash), [`PartialEq`], and [`PartialOrd`] (ex: `reflect_clone` and
/// `reflect_partial_eq`). Each operation first uses the function in the type's
/// [`TypeVTable`](crate::TypeVTable), which is registered using attributes like
/// `#[reflect(Debug)]` or `#[reflect(Debug(custom_debug_fn))]`. If the type doesn't register one, a
/// structural implementation based on its [`Kind`] is used.
///
/// Structural implementations fail (return `None`) if any nested value does not support the
/// operation.
///
/// # Safety
///
/// [`Reflect::ty`] must accurately describe `Self` (see [`Type::new`]).
//
// Design Notes:
// - Keep this small ... it will be implemented for many types. Don't tack on a bunch of convenience
//   methods here
pub unsafe trait Reflect: Any {
    /// Returns the [`Type`] of this value.
    fn ty(&self) -> &'static Type;
}

// SAFETY: `Typed::TYPE` has the same requirements as `Reflect::ty`
unsafe impl<T: Typed> Reflect for T {
    fn ty(&self) -> &'static Type {
        T::TYPE
    }
}

impl dyn Reflect {
    /// Returns a reference to this value that matches its [`Kind`].
    pub fn kind_ref(&self) -> KindRef<'_> {
        let ptr = self.ptr();
        // SAFETY: `ptr` is a valid, initialized value that matches the type described by
        // `self.ty()`, so it matches every kind description it contains
        unsafe {
            match self.ty().kind() {
                Kind::Struct(struct_type) => KindRef::Struct(StructRef::new(ptr, struct_type)),
                Kind::Enum(enum_type) => KindRef::Enum(EnumRef::new(ptr, enum_type)),
                Kind::List(list_type) => KindRef::List(ListRef::new(ptr, list_type)),
                Kind::Array(array_type) => KindRef::Array(ArrayRef::new(ptr, array_type)),
                Kind::Map(map_type) => KindRef::Map(MapRef::new(ptr, map_type)),
                Kind::Set(set_type) => KindRef::Set(SetRef::new(ptr, set_type)),
                Kind::Primitive(primitive) => KindRef::Primitive(primitive),
            }
        }
    }

    /// Returns a mutable reference to this value that matches its [`Kind`].
    pub fn kind_mut(&mut self) -> KindMut<'_> {
        let ty = self.ty();
        let ptr = self.ptr_mut();
        // SAFETY: `ptr` is a valid, initialized value that matches the type described by
        // `self.ty()`, so it matches every kind description it contains
        unsafe {
            match ty.kind() {
                Kind::Struct(struct_type) => KindMut::Struct(StructMut::new(ptr, struct_type)),
                Kind::Enum(enum_type) => KindMut::Enum(EnumMut::new(ptr, enum_type)),
                Kind::List(list_type) => KindMut::List(ListMut::new(ptr, list_type)),
                Kind::Array(array_type) => KindMut::Array(ArrayMut::new(ptr, array_type)),
                Kind::Map(map_type) => KindMut::Map(MapMut::new(ptr, map_type)),
                Kind::Set(set_type) => KindMut::Set(SetMut::new(ptr, set_type)),
                Kind::Primitive(primitive) => KindMut::Primitive(primitive),
            }
        }
    }

    /// Returns whether this value is a `T`.
    #[inline]
    pub fn is<T: Any>(&self) -> bool {
        self.ty().type_id() == TypeId::of::<T>()
    }

    /// Returns this value as a `T` if it is one.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        // SAFETY: `is` guarantees that this value is a `T` (see `Typed`)
        self.is::<T>()
            .then(|| unsafe { &*ptr::from_ref(self).cast::<T>() })
    }

    /// Returns this value as a `T` if it is one.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        // SAFETY: `is` guarantees that this value is a `T` (see `Typed`)
        self.is::<T>()
            .then(|| unsafe { &mut *ptr::from_mut(self).cast::<T>() })
    }

    /// Returns this value as a `T` if it is one. Otherwise, it is returned
    /// unchanged in an `Err`.
    pub fn downcast<T: Any>(self: Box<Self>) -> Result<Box<T>, Box<dyn Reflect>> {
        if !self.is::<T>() {
            return Err(self);
        }
        // SAFETY: `is` guarantees that this value is a `T` (see `Typed`), so the allocation is
        // valid for a `Box<T>`.
        Ok(unsafe { Box::from_raw(Box::into_raw(self).cast::<T>()) })
    }

    /// Downcasts this value to `T`, moving it out of the box. If this is not a `T`, it
    /// is returned unchanged in an `Err`.
    pub fn take<T: Any>(self: Box<Self>) -> Result<T, Box<dyn Reflect>> {
        self.downcast::<T>().map(|value| *value)
    }

    /// Replaces this value with `value`, returning the previous value. Fails (returning `value`
    /// unchanged in an `Err`) if `value` is not the same type as this value.
    pub fn replace(
        &mut self,
        mut value: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        if value.ty().type_id() != self.ty().type_id() {
            return Err(value);
        }
        let size = Layout::for_value(self).size();
        // SAFETY: both values have the same type, so they have the same layout. They are separate
        // allocations, so they don't overlap. Swapping their bytes moves each value into the other's location.
        unsafe {
            ptr::swap_nonoverlapping(self.ptr_mut().as_ptr(), value.ptr_mut().as_ptr(), size);
        }
        Ok(value)
    }

    /// Sets this value to `value`, dropping the previous value. Fails (returning `value`
    /// unchanged in an `Err`) if `value` is not the same type as this value.
    pub fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        self.replace(value).map(drop)
    }

    /// Returns a [`Ptr`] to this value.
    #[inline]
    pub fn ptr(&self) -> Ptr<'_> {
        Ptr::from(self)
    }

    /// Returns a [`PtrMut`] to this value.
    #[inline]
    pub fn ptr_mut(&mut self) -> PtrMut<'_> {
        PtrMut::from(self)
    }

    /// Returns a [`StructRef`], if this type is a [`Kind::Struct`].
    pub fn struct_ref(&self) -> Option<StructRef<'_>> {
        let Kind::Struct(struct_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr() is a valid, initialized type that matches the type described in struct_type
        Some(unsafe { StructRef::new(self.ptr(), struct_type) })
    }

    /// Returns a [`StructMut`], if this type is a [`Kind::Struct`].
    pub fn struct_mut(&mut self) -> Option<StructMut<'_>> {
        let Kind::Struct(struct_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr_mut() is a valid, initialized type that matches the type described in struct_type
        Some(unsafe { StructMut::new(self.ptr_mut(), struct_type) })
    }

    /// Returns an [`EnumRef`], if this type is a [`Kind::Enum`].
    pub fn enum_ref(&self) -> Option<EnumRef<'_>> {
        let Kind::Enum(enum_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr() is a valid, initialized type that matches the type described in enum_type
        Some(unsafe { EnumRef::new(self.ptr(), enum_type) })
    }

    /// Returns an [`EnumMut`], if this type is a [`Kind::Enum`].
    pub fn enum_mut(&mut self) -> Option<EnumMut<'_>> {
        let Kind::Enum(enum_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr_mut() is a valid, initialized type that matches the type described in enum_type
        Some(unsafe { EnumMut::new(self.ptr_mut(), enum_type) })
    }

    /// Returns a [`ListRef`], if this type is a [`Kind::List`].
    pub fn list_ref(&self) -> Option<ListRef<'_>> {
        let Kind::List(list_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr() is a valid, initialized type that matches the type described in list_type
        Some(unsafe { ListRef::new(self.ptr(), list_type) })
    }

    /// Returns a [`ListMut`], if this type is a [`Kind::List`].
    pub fn list_mut(&mut self) -> Option<ListMut<'_>> {
        let Kind::List(list_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr_mut() is a valid, initialized type that matches the type described in list_type
        Some(unsafe { ListMut::new(self.ptr_mut(), list_type) })
    }

    /// Returns an [`ArrayRef`], if this type is a [`Kind::Array`].
    pub fn array_ref(&self) -> Option<ArrayRef<'_>> {
        let Kind::Array(array_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr() is a valid, initialized type that matches the type described in array_type
        Some(unsafe { ArrayRef::new(self.ptr(), array_type) })
    }

    /// Returns an [`ArrayMut`], if this type is a [`Kind::Array`].
    pub fn array_mut(&mut self) -> Option<ArrayMut<'_>> {
        let Kind::Array(array_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr_mut() is a valid, initialized type that matches the type described in array_type
        Some(unsafe { ArrayMut::new(self.ptr_mut(), array_type) })
    }

    /// Returns an [`MapRef`], if this type is a [`Kind::Map`].
    pub fn map_ref(&self) -> Option<MapRef<'_>> {
        let Kind::Map(map_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr() is a valid, initialized type that matches the type described in map_type
        Some(unsafe { MapRef::new(self.ptr(), map_type) })
    }

    /// Returns an [`MapMut`], if this type is a [`Kind::Map`].
    pub fn map_mut(&mut self) -> Option<MapMut<'_>> {
        let Kind::Map(map_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr_mut() is a valid, initialized type that matches the type described in map_type
        Some(unsafe { MapMut::new(self.ptr_mut(), map_type) })
    }

    /// Returns an [`SetRef`], if this type is a [`Kind::Set`].
    pub fn set_ref(&self) -> Option<SetRef<'_>> {
        let Kind::Set(set_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr() is a valid, initialized type that matches the type described in set_type
        Some(unsafe { SetRef::new(self.ptr(), set_type) })
    }

    /// Returns an [`SetMut`], if this type is a [`Kind::Set`].
    pub fn set_mut(&mut self) -> Option<SetMut<'_>> {
        let Kind::Set(set_type) = self.ty().kind() else {
            return None;
        };
        // SAFETY: self.ptr_mut() is a valid, initialized type that matches the type described in set_type
        Some(unsafe { SetMut::new(self.ptr_mut(), set_type) })
    }
}

/// `dyn Reflect + Send + Sync` dereferences to `dyn Reflect`, which provides access to its
/// methods (ex: `downcast_ref` and `struct_ref`).
impl Deref for dyn Reflect + Send + Sync {
    type Target = dyn Reflect;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for dyn Reflect + Send + Sync {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

impl dyn Reflect + Send + Sync {
    /// Returns this value as a `T` if it is one. Otherwise, it is returned
    /// unchanged in an `Err`.
    pub fn downcast<T: Any>(self: Box<Self>) -> Result<Box<T>, Box<dyn Reflect + Send + Sync>> {
        if !self.is::<T>() {
            return Err(self);
        }
        let value: Box<dyn Reflect> = self;
        Ok(value
            .downcast::<T>()
            .unwrap_or_else(|_| unreachable!("the value was checked to be a `T`")))
    }

    /// Downcasts this value to `T`, moving it out of the box. If this is not a `T`, it
    /// is returned unchanged in an `Err`.
    pub fn take<T: Any>(self: Box<Self>) -> Result<T, Box<dyn Reflect + Send + Sync>> {
        self.downcast::<T>().map(|value| *value)
    }

    /// Returns a clone of this value (see `reflect_clone` on `dyn Reflect`). Unlike that method,
    /// the clone is known to be [`Send`] and [`Sync`].
    pub fn reflect_clone(&self) -> Option<Box<dyn Reflect + Send + Sync>> {
        let clone = (**self).reflect_clone()?;
        let raw = Box::into_raw(clone);
        // SAFETY: a clone has the type of the value it was cloned from. This value is `Send` and
        // `Sync`, so the clone is too. Auto traits don't affect the vtable, so the
        // pointers have the same representation.
        Some(unsafe {
            Box::from_raw(core::mem::transmute::<
                *mut dyn Reflect,
                *mut (dyn Reflect + Send + Sync),
            >(raw))
        })
    }
}

impl Debug for dyn Reflect + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::Reflect;
    use alloc::{boxed::Box, string::String};

    #[test]
    fn set_and_replace() {
        let mut value = String::from("a");
        let reflect = &mut value as &mut dyn Reflect;
        let old = reflect.replace(Box::new(String::from("b"))).unwrap();
        assert_eq!(old.downcast_ref::<String>().unwrap(), "a");
        reflect.set(Box::new(String::from("c"))).unwrap();
        assert!(reflect.set(Box::new(1u32)).is_err());
        assert_eq!(value, "c");
    }

    #[test]
    fn send_sync() {
        let value: Box<dyn Reflect + Send + Sync> = Box::new(String::from("a"));
        assert_eq!(value.downcast_ref::<String>().unwrap(), "a");
        let clone: Box<dyn Reflect + Send + Sync> = value.reflect_clone().unwrap();
        assert_eq!(alloc::format!("{clone:?}"), "\"a\"");
        assert_eq!(clone.take::<String>().unwrap(), "a");
        assert!(value.downcast::<u32>().is_err());
    }
}
