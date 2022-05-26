//! Helpers for working with Bevy reflection.

use crate::TypeInfo;
use bevy_utils::HashMap;
use once_cell::race::OnceBox;
use parking_lot::RwLock;
use std::any::{Any, TypeId};

/// A container for [`TypeInfo`], allowing instances to be stored statically.
///
/// Under the hood, this manages a [`once_cell`] for either of the two possible types:
/// `Generic` and `NonGeneric`.
///
/// ## Non-Generic
///
/// For non-generic types, [`TypeInfoCell`] should be initialized via the [`non_generic`]
/// method. This should be much more performant than the generic alternative, so favor
/// this variant whenever possible.
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{Reflect, ReflectMut, ReflectRef, Typed, TypeInfo, ValueInfo};
/// use bevy_reflect::utility::TypeInfoCell;
///
/// struct Foo;
///
/// impl Typed for Foo {
///   fn type_info() -> &'static TypeInfo {
///     static CELL: TypeInfoCell = TypeInfoCell::non_generic();
///     CELL.get_or_insert::<Self, _>(|| {
///       let info = ValueInfo::new::<Self>();
///       TypeInfo::Value(info)
///     })
///   }
/// }
/// #
/// # unsafe impl Reflect for Foo {
/// #   fn type_name(&self) -> &str { todo!() }
/// #   fn get_type_info(&self) -> &'static TypeInfo { todo!() }
/// #   fn any(&self) -> &dyn Any { todo!() }
/// #   fn any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #   fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #   fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #   fn apply(&mut self, value: &dyn Reflect) { todo!() }
/// #   fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// #   fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #   fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #   fn clone_value(&self) -> Box<dyn Reflect> { todo!() }
/// # }
/// ```
///
/// ## Generic
///
/// For generic types, [`TypeInfoCell`] should be initialized via the [`generic`]
/// method. This will store multiple instances of [`TypeInfo`], accessible by [`TypeId`].
///
/// This allows `Foo<T>` to use the same [`TypeInfoCell`] for monomorphized type.
///
/// ```
/// # use std::any::Any;
/// # use std::marker::PhantomData;
/// # use bevy_reflect::{Reflect, ReflectMut, ReflectRef, Typed, TypeInfo, ValueInfo};
/// use bevy_reflect::utility::TypeInfoCell;
///
/// struct Foo<T: Reflect>(PhantomData<T>);
///
/// impl<T: Reflect> Typed for Foo<T> {
///   fn type_info() -> &'static TypeInfo {
///     static CELL: TypeInfoCell = TypeInfoCell::generic();
///     CELL.get_or_insert::<Self, _>(|| {
///       let info = ValueInfo::new::<Self>();
///       TypeInfo::Value(info)
///     })
///   }
/// }
/// #
/// # unsafe impl<T: Reflect> Reflect for Foo<T> {
/// #   fn type_name(&self) -> &str { todo!() }
/// #   fn get_type_info(&self) -> &'static TypeInfo { todo!() }
/// #   fn any(&self) -> &dyn Any { todo!() }
/// #   fn any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #   fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #   fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #   fn apply(&mut self, value: &dyn Reflect) { todo!() }
/// #   fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// #   fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #   fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #   fn clone_value(&self) -> Box<dyn Reflect> { todo!() }
/// # }
/// ```
///
/// [`once_cell`]: https://docs.rs/once_cell/latest/once_cell/
/// [`non_generic`]: TypeInfoCell::non_generic
/// [`generic`]: TypeInfoCell::generic
pub struct TypeInfoCell(TypeInfoCellType);

impl TypeInfoCell {
    /// Initialize a [`TypeInfoCell`] for non-generic types.
    pub const fn non_generic() -> Self {
        Self(TypeInfoCellType::NonGeneric(OnceBox::new()))
    }

    /// Initialize a [`TypeInfoCell`] for generic types.
    pub const fn generic() -> Self {
        Self(TypeInfoCellType::Generic(OnceBox::new()))
    }

    /// Returns a reference to the [`TypeInfo`] stored in the cell.
    ///
    /// If there is no [`TypeInfo`] found, a new one will be generated from the given function.
    ///
    /// # Generics
    ///
    /// Generic types, such as `Vec<T>`, store a mapping of [`TypeIds`] to [`TypeInfos`]. This
    /// method will then return the correct [`TypeInfo`] reference for the given type `T`.
    ///
    /// [`TypeIds`]: std::any::TypeId
    /// [`TypeInfos`]: TypeInfo
    pub fn get_or_insert<T, F>(&self, f: F) -> &TypeInfo
    where
        T: Any + ?Sized,
        F: FnOnce() -> TypeInfo,
    {
        match &self.0 {
            TypeInfoCellType::NonGeneric(once) => once.get_or_init(|| Box::new(f())),
            TypeInfoCellType::Generic(once) => {
                let type_id = TypeId::of::<T>();
                let mapping = once.get_or_init(|| Box::new(RwLock::default()));
                if let Some(info) = mapping.read().get(&type_id) {
                    return info;
                }

                mapping.write().entry(type_id).or_insert_with(|| {
                    // We leak here in order to obtain a `&'static` reference.
                    // Otherwise, we won't be able to return a reference due to the `RwLock`.
                    // This should be okay, though, since we expect it to remain statically
                    // available over the course of the application.
                    Box::leak(Box::new(f()))
                })
            }
        }
    }
}

enum TypeInfoCellType {
    NonGeneric(OnceBox<TypeInfo>),
    Generic(OnceBox<RwLock<HashMap<TypeId, &'static TypeInfo>>>),
}
