use serde::Serializer;

use crate::{PartialReflect, TypeRegistry};

/// Allows overriding the default serialization behavior of
/// [`ReflectSerializer`] and [`TypedReflectSerializer`] for specific values.
///
/// When serializing a reflected value, you may want to override the default
/// behavior and use your own logic for serialization. This logic may also be
/// context-dependent, and only apply for a single use of your
/// [`ReflectSerializer`]. To achieve this, you can create a processor and pass
/// it into your serializer.
///
/// Whenever the serializer attempts to serialize a value, it will first call
/// [`try_serialize`] on your processor, which may take ownership of the
/// serializer and write into the serializer (successfully or not), or return
/// ownership of the serializer back, and continue with the default logic.
///
/// The deserialization equivalent of this is [`ReflectDeserializerProcessor`].
///
/// # Compared to [`SerializeWithRegistry`]
///
/// [`SerializeWithRegistry`] allows you to define how your type will be
/// serialized by a [`TypedReflectSerializer`], given the extra context of the
/// [`TypeRegistry`]. If your type can be serialized entirely using that, then
/// you should prefer implementing that trait instead of using a processor.
///
/// However, you may need more context-dependent data which is only present in
/// the scope where you create the [`TypedReflectSerializer`]. For example, if
/// you need to use a reference to a value while serializing, then there is no
/// way to do this with [`SerializeWithRegistry`] as you can't pass that
/// reference into anywhere. This is where a processor is useful, as the
/// processor can capture local variables.
///
/// A [`ReflectSerializerProcessor`] always takes priority over a
/// [`SerializeWithRegistry`] implementation, so this is also useful for
/// overriding serialization behavior if you need to do something custom.
///
/// # Examples
///
/// Serializing a reflected value when saving an asset to disk, and replacing
/// asset handles with the handle path (if it has one):
///
/// ```
/// # use core::any::Any;
/// # use serde::Serialize;
/// # use bevy_reflect::{PartialReflect, Reflect, TypeData, TypeRegistry};
/// # use bevy_reflect::serde::{ReflectSerializer, ReflectSerializerProcessor};
/// #
/// # #[derive(Debug, Clone, Reflect)]
/// # struct Handle<T>(T);
/// # #[derive(Debug, Clone, Reflect)]
/// # struct Mesh;
/// #
/// # struct ReflectHandle;
/// # impl TypeData for ReflectHandle {
/// #     fn clone_type_data(&self) -> Box<dyn TypeData> {
/// #         unimplemented!()
/// #     }
/// # }
/// # impl ReflectHandle {
/// #     fn downcast_handle_untyped(&self, handle: &(dyn Any + 'static)) -> Option<UntypedHandle> {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// # #[derive(Debug, Clone)]
/// # struct UntypedHandle;
/// # impl UntypedHandle {
/// #     fn path(&self) -> Option<&str> {
/// #         unimplemented!()
/// #     }
/// # }
/// # type AssetError = Box<dyn core::error::Error>;
/// #
/// #[derive(Debug, Clone, Reflect)]
/// struct MyAsset {
///     name: String,
///     mesh: Handle<Mesh>,
/// }
///
/// struct HandleProcessor;
///
/// impl ReflectSerializerProcessor for HandleProcessor {
///     fn try_serialize<S>(
///         &self,
///         value: &dyn PartialReflect,
///         registry: &TypeRegistry,
///         serializer: S,
///     ) -> Result<Result<S::Ok, S>, S::Error>
///     where
///         S: serde::Serializer,
///     {
///         let Some(value) = value.try_as_reflect() else {
///             // we don't have any info on this type; do the default serialization logic
///             return Ok(Err(serializer));
///         };
///         let type_id = value.reflect_type_info().type_id();
///         let Some(reflect_handle) = registry.get_type_data::<ReflectHandle>(type_id) else {
///             // this isn't a `Handle<T>`
///             return Ok(Err(serializer));
///         };
///
///         let untyped_handle = reflect_handle
///             .downcast_handle_untyped(value.as_any())
///             .unwrap();
///         if let Some(path) = untyped_handle.path() {
///             Ok(Ok(serializer.serialize_str(path)?))
///         } else {
///             Ok(Ok(serializer.serialize_unit()?))
///         }
///     }
/// }
///
/// fn save(type_registry: &TypeRegistry, asset: &MyAsset) -> Result<String, AssetError> {
///     let mut asset_string = String::new();
///
///     let processor = HandleProcessor;
///     let serializer = ReflectSerializer::with_processor(asset, type_registry, &processor);
///     let mut ron_serializer = ron::Serializer::new(&mut asset_string, None)?;
///
///     serializer.serialize(&mut ron_serializer)?;
///     Ok(asset_string)
/// }
/// ```
///
/// [`ReflectSerializer`]: crate::serde::ReflectSerializer
/// [`TypedReflectSerializer`]: crate::serde::TypedReflectSerializer
/// [`try_serialize`]: Self::try_serialize
/// [`SerializeWithRegistry`]: crate::serde::SerializeWithRegistry
/// [`ReflectDeserializerProcessor`]: crate::serde::ReflectDeserializerProcessor
pub trait ReflectSerializerProcessor {
    /// Attempts to serialize the value which a [`TypedReflectSerializer`] is
    /// currently looking at.
    ///
    /// If you want to override the default serialization, return
    /// `Ok(Ok(value))` with an `Ok` output from the serializer.
    ///
    /// If you don't want to override the serialization, return ownership of
    /// the serializer back via `Ok(Err(serializer))`.
    ///
    /// You can use the type registry to read info about the type you're
    /// serializing, or just try to downcast the value directly:
    ///
    /// ```
    /// # use bevy_reflect::{TypeRegistration, TypeRegistry, PartialReflect};
    /// # use bevy_reflect::serde::ReflectSerializerProcessor;
    /// # use core::any::TypeId;
    /// struct I32AsStringProcessor;
    ///
    /// impl ReflectSerializerProcessor for I32AsStringProcessor {
    ///     fn try_serialize<S>(
    ///         &self,
    ///         value: &dyn PartialReflect,
    ///         registry: &TypeRegistry,
    ///         serializer: S,
    ///     ) -> Result<Result<S::Ok, S>, S::Error>
    ///     where
    ///         S: serde::Serializer
    ///     {
    ///         if let Some(value) = value.try_downcast_ref::<i32>() {
    ///             let value_as_string = format!("{value:?}");
    ///             Ok(Ok(serializer.serialize_str(&value_as_string)?))
    ///         } else {
    ///             // Not an `i32`, just do the default serialization
    ///             Ok(Err(serializer))
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// [`TypedReflectSerializer`]: crate::serde::TypedReflectSerializer
    /// [`Reflect`]: crate::Reflect
    fn try_serialize<S>(
        &self,
        value: &dyn PartialReflect,
        registry: &TypeRegistry,
        serializer: S,
    ) -> Result<Result<S::Ok, S>, S::Error>
    where
        S: Serializer;
}

impl ReflectSerializerProcessor for () {
    fn try_serialize<S>(
        &self,
        _value: &dyn PartialReflect,
        _registry: &TypeRegistry,
        serializer: S,
    ) -> Result<Result<S::Ok, S>, S::Error>
    where
        S: Serializer,
    {
        Ok(Err(serializer))
    }
}
