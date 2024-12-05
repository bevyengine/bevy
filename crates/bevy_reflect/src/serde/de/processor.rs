use crate::{PartialReflect, TypeRegistration, TypeRegistry};
use alloc::boxed::Box;

/// Allows overriding the default deserialization behavior of
/// [`ReflectDeserializer`] and [`TypedReflectDeserializer`] for specific
/// [`TypeRegistration`]s.
///
/// When deserializing a reflected value, you may want to override the default
/// behavior and use your own logic for deserialization. This logic may also
/// be context-dependent, and only apply for a single use of your
/// [`ReflectDeserializer`]. To achieve this, you can create a processor and
/// pass it in to your deserializer.
///
/// Whenever the deserializer attempts to deserialize a value, it will first
/// call [`try_deserialize`] on your processor, which may take ownership of the
/// deserializer and give back a [`Box<dyn PartialReflect>`], or return
/// ownership of the deserializer back, and continue with the default logic.
///
/// The serialization equivalent of this is [`ReflectSerializerProcessor`].
///
/// # Compared to [`DeserializeWithRegistry`]
///
/// [`DeserializeWithRegistry`] allows you to define how your type will be
/// deserialized by a [`TypedReflectDeserializer`], given the extra context of
/// the [`TypeRegistry`]. If your type can be deserialized entirely from that,
/// then you should prefer implementing that trait instead of using a processor.
///
/// However, you may need more context-dependent data which is only present in
/// the scope where you create the [`TypedReflectDeserializer`]. For example, in
/// an asset loader, the `&mut LoadContext` you get is only valid from within
/// the `load` function. This is where a processor is useful, as the processor
/// can capture local variables.
///
/// A [`ReflectDeserializerProcessor`] always takes priority over a
/// [`DeserializeWithRegistry`] implementation, so this is also useful for
/// overriding deserialization behavior if you need to do something custom.
///
/// # Examples
///
/// Deserializing a reflected value in an asset loader, and replacing asset
/// handles with a loaded equivalent:
///
/// ```
/// # use bevy_reflect::serde::{ReflectDeserializer, ReflectDeserializerProcessor};
/// # use bevy_reflect::{PartialReflect, Reflect, TypeData, TypeRegistration, TypeRegistry};
/// # use serde::de::{DeserializeSeed, Deserializer, Visitor};
/// # use std::marker::PhantomData;
/// #
/// # #[derive(Debug, Clone, Reflect)]
/// # struct LoadedUntypedAsset;
/// # #[derive(Debug, Clone, Reflect)]
/// # struct Handle<T: Reflect>(T);
/// # #[derive(Debug, Clone, Reflect)]
/// # struct Mesh;
/// #
/// # struct LoadContext;
/// # impl LoadContext {
/// #     fn load(&mut self) -> &mut Self { unimplemented!() }
/// #     fn with_asset_type_id(&mut self, (): ()) -> &mut Self { unimplemented!() }
/// #     fn untyped(&mut self) -> &mut Self { unimplemented!() }
/// #     fn load_asset(&mut self, (): ()) -> Handle<LoadedUntypedAsset> { unimplemented!() }
/// # }
/// #
/// # struct ReflectHandle;
/// # impl TypeData for ReflectHandle {
/// #     fn clone_type_data(&self) -> Box<dyn TypeData> {
/// #         unimplemented!()
/// #     }
/// # }
/// # impl ReflectHandle {
/// #     fn asset_type_id(&self) {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// # struct AssetPathVisitor;
/// # impl<'de> Visitor<'de> for AssetPathVisitor {
/// #     type Value = ();
/// #     fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result { unimplemented!() }
/// # }
/// # type AssetError = Box<dyn core::error::Error>;
/// #[derive(Debug, Clone, Reflect)]
/// struct MyAsset {
///     name: String,
///     mesh: Handle<Mesh>,
/// }
///
/// fn load(
///     asset_bytes: &[u8],
///     type_registry: &TypeRegistry,
///     load_context: &mut LoadContext,
/// ) -> Result<MyAsset, AssetError> {
///     struct HandleProcessor<'a> {
///         load_context: &'a mut LoadContext,
///     }
///
///     impl ReflectDeserializerProcessor for HandleProcessor<'_> {
///         fn try_deserialize<'de, D>(
///             &mut self,
///             registration: &TypeRegistration,
///             _registry: &TypeRegistry,
///             deserializer: D,
///         ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
///         where
///             D: Deserializer<'de>,
///         {
///             let Some(reflect_handle) = registration.data::<ReflectHandle>() else {
///                 // we don't want to deserialize this - give the deserializer back
///                 return Ok(Err(deserializer));
///             };
///
///             let asset_type_id = reflect_handle.asset_type_id();
///             let asset_path = deserializer.deserialize_str(AssetPathVisitor)?;
///
///             let handle: Handle<LoadedUntypedAsset> = self.load_context
///                 .load()
///                 .with_asset_type_id(asset_type_id)
///                 .untyped()
///                 .load_asset(asset_path);
///             # let _: Result<_, ()> = {
///             Ok(Box::new(handle))
///             # };
///             # unimplemented!()
///         }
///     }
///
///     let mut ron_deserializer = ron::Deserializer::from_bytes(asset_bytes)?;
///     let mut processor = HandleProcessor { load_context };
///     let reflect_deserializer =
///         ReflectDeserializer::with_processor(type_registry, &mut processor);
///     let asset = reflect_deserializer.deserialize(&mut ron_deserializer)?;
///     # unimplemented!()
/// }
/// ```
///
/// [`ReflectDeserializer`]: crate::serde::ReflectDeserializer
/// [`TypedReflectDeserializer`]: crate::serde::TypedReflectDeserializer
/// [`try_deserialize`]: Self::try_deserialize
/// [`DeserializeWithRegistry`]: crate::serde::DeserializeWithRegistry
/// [`ReflectSerializerProcessor`]: crate::serde::ReflectSerializerProcessor
pub trait ReflectDeserializerProcessor {
    /// Attempts to deserialize the value which a [`TypedReflectDeserializer`]
    /// is currently looking at, and knows the type of.
    ///
    /// If you've read the `registration` and want to override the default
    /// deserialization, return `Ok(Ok(value))` with the boxed reflected value
    /// that you want to assign this value to. The type inside the box must
    /// be the same one as the `registration` is for, otherwise future
    /// reflection operations (such as using [`FromReflect`] to convert the
    /// resulting [`Box<dyn PartialReflect>`] into a concrete type) will fail.
    ///
    /// If you don't want to override the deserialization, return ownership of
    /// the deserializer back via `Ok(Err(deserializer))`.
    ///
    /// Note that, if you do want to return a value, you *must* read from the
    /// deserializer passed to this function (you are free to ignore the result
    /// though). Otherwise, the deserializer will be in an inconsistent state,
    /// and future value parsing will fail.
    ///
    /// # Examples
    ///
    /// Correct way to return a constant value (not using any output from the
    /// deserializer):
    ///
    /// ```
    /// # use bevy_reflect::{TypeRegistration, PartialReflect, TypeRegistry};
    /// # use bevy_reflect::serde::ReflectDeserializerProcessor;
    /// # use core::any::TypeId;
    /// use serde::de::IgnoredAny;
    ///
    /// struct ConstantI32Processor;
    ///
    /// impl ReflectDeserializerProcessor for ConstantI32Processor {
    ///     fn try_deserialize<'de, D>(
    ///         &mut self,
    ///         registration: &TypeRegistration,
    ///         _registry: &TypeRegistry,
    ///         deserializer: D,
    ///     ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
    ///     where
    ///         D: serde::Deserializer<'de>
    ///     {
    ///         if registration.type_id() == TypeId::of::<i32>() {
    ///             _ = deserializer.deserialize_ignored_any(IgnoredAny);
    ///             Ok(Ok(Box::new(42_i32)))
    ///         } else {
    ///             Ok(Err(deserializer))
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// [`TypedReflectDeserializer`]: crate::serde::TypedReflectDeserializer
    /// [`FromReflect`]: crate::FromReflect
    fn try_deserialize<'de, D>(
        &mut self,
        registration: &TypeRegistration,
        registry: &TypeRegistry,
        deserializer: D,
    ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
    where
        D: serde::Deserializer<'de>;
}

impl ReflectDeserializerProcessor for () {
    fn try_deserialize<'de, D>(
        &mut self,
        _registration: &TypeRegistration,
        _registry: &TypeRegistry,
        deserializer: D,
    ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Err(deserializer))
    }
}
