use crate::{
    array::DynamicArray, enums::DynamicEnum, list::DynamicList, map::DynamicMap, set::DynamicSet,
    structs::DynamicStruct, tuple::DynamicTuple, tuple_struct::DynamicTupleStruct, PartialReflect,
    Reflect, TypeInfo, TypePath,
};

/// A static accessor to compile-time type information.
///
/// This trait is automatically implemented by the [`#[derive(Reflect)]`](derive@crate::Reflect) macro
/// and allows type information to be processed without an instance of that type.
///
/// If you need to use this trait as a generic bound along with other reflection traits,
/// for your convenience, consider using [`Reflectable`] instead.
///
/// # Implementing
///
/// While it is recommended to leave implementing this trait to the `#[derive(Reflect)]` macro,
/// it is possible to implement this trait manually. If a manual implementation is needed,
/// you _must_ ensure that the information you provide is correct, otherwise various systems that
/// rely on this trait may fail in unexpected ways.
///
/// Implementors may have difficulty in generating a reference to [`TypeInfo`] with a static
/// lifetime. Luckily, this crate comes with some [utility] structs, to make generating these
/// statics much simpler.
///
/// # Example
///
/// ```
/// # use core::any::Any;
/// # use bevy_reflect::{DynamicTypePath, NamedField, PartialReflect, Reflect, ReflectMut, ReflectOwned, ReflectRef, structs::StructInfo, TypeInfo, TypePath, OpaqueInfo, ApplyError};
/// # use bevy_reflect::utility::NonGenericTypeInfoCell;
/// use bevy_reflect::Typed;
///
/// struct MyStruct {
///   foo: usize,
///   bar: (f32, f32)
/// }
///
/// impl Typed for MyStruct {
///   fn type_info() -> &'static TypeInfo {
///     static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
///     CELL.get_or_set(|| {
///       let fields = [
///         NamedField::new::<usize >("foo"),
///         NamedField::new::<(f32, f32) >("bar"),
///       ];
///       let info = StructInfo::new::<Self>(&fields);
///       TypeInfo::Struct(info)
///     })
///   }
/// }
///
/// # impl TypePath for MyStruct {
/// #     fn type_path() -> &'static str { todo!() }
/// #     fn short_type_path() -> &'static str { todo!() }
/// # }
/// # impl PartialReflect for MyStruct {
/// #     fn get_represented_type_info(&self) -> Option<&'static TypeInfo> { todo!() }
/// #     fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> { todo!() }
/// #     fn as_partial_reflect(&self) -> &dyn PartialReflect { todo!() }
/// #     fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect { todo!() }
/// #     fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> { todo!() }
/// #     fn try_as_reflect(&self) -> Option<&dyn Reflect> { todo!() }
/// #     fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> { todo!() }
/// #     fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> { todo!() }
/// #     fn reflect_ref(&self) -> ReflectRef<'_> { todo!() }
/// #     fn reflect_mut(&mut self) -> ReflectMut<'_> { todo!() }
/// #     fn reflect_owned(self: Box<Self>) -> ReflectOwned { todo!() }
/// # }
/// # impl Reflect for MyStruct {
/// #     fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #     fn as_any(&self) -> &dyn Any { todo!() }
/// #     fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #     fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> { todo!() }
/// #     fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #     fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #     fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// # }
/// ```
///
/// [`Reflectable`]: crate::Reflectable
/// [utility]: crate::utility
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Typed` so cannot provide static type information",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait Typed: Reflect + TypePath {
    /// Returns the compile-time [info] for the underlying type.
    ///
    /// [info]: TypeInfo
    fn type_info() -> &'static TypeInfo;
}

/// Dynamic dispatch for [`Typed`].
///
/// Since this is a supertrait of [`Reflect`] its methods can be called on a `dyn Reflect`.
///
/// [`Reflect`]: crate::Reflect
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not provide dynamic type information through reflection",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait DynamicTyped {
    /// See [`Typed::type_info`].
    fn reflect_type_info(&self) -> &'static TypeInfo;
}

impl<T: Typed> DynamicTyped for T {
    #[inline]
    fn reflect_type_info(&self) -> &'static TypeInfo {
        Self::type_info()
    }
}

/// A wrapper trait around [`Typed`].
///
/// This trait is used to provide a way to get compile-time type information for types that
/// do implement `Typed` while also allowing for types that do not implement `Typed` to be used.
/// It's used instead of `Typed` directly to avoid making dynamic types also
/// implement `Typed` in order to be used as active fields.
///
/// This trait has a blanket implementation for all types that implement `Typed`
/// and manual implementations for all dynamic types (which simply return `None`).
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Typed` so cannot provide static type information",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait MaybeTyped: PartialReflect {
    /// Returns the compile-time [info] for the underlying type, if it exists.
    ///
    /// [info]: TypeInfo
    fn maybe_type_info() -> Option<&'static TypeInfo> {
        None
    }
}

impl<T: Typed> MaybeTyped for T {
    fn maybe_type_info() -> Option<&'static TypeInfo> {
        Some(T::type_info())
    }
}

impl MaybeTyped for DynamicEnum {}

impl MaybeTyped for DynamicTupleStruct {}

impl MaybeTyped for DynamicStruct {}

impl MaybeTyped for DynamicMap {}

impl MaybeTyped for DynamicSet {}

impl MaybeTyped for DynamicList {}

impl MaybeTyped for DynamicArray {}

impl MaybeTyped for DynamicTuple {}
