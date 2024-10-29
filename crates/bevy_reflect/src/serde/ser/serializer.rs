#[cfg(feature = "debug_stack")]
use crate::serde::ser::error_utils::TYPE_INFO_STACK;
use crate::{
    serde::ser::{
        arrays::ArraySerializer, custom_serialization::try_custom_serialize, enums::EnumSerializer,
        error_utils::make_custom_error, lists::ListSerializer, maps::MapSerializer,
        sets::SetSerializer, structs::StructSerializer, tuple_structs::TupleStructSerializer,
        tuples::TupleSerializer,
    },
    PartialReflect, ReflectRef, TypeRegistry,
};
use serde::{ser::SerializeMap, Serialize, Serializer};

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
///             // we don't have any info on this type; do the default logic
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
///             serializer.serialize_str(path).map(Ok)
///         } else {
///             serializer.serialize_unit().map(Ok)
///         }
///     }
/// }
///
/// fn save(type_registry: &TypeRegistry, asset: &MyAsset) -> Result<Vec<u8>, AssetError> {
///     let mut asset_bytes = Vec::new();
///
///     let processor = HandleProcessor;
///     let serializer = ReflectSerializer::with_processor(asset, type_registry, &processor);
///     let mut ron_serializer = ron::Serializer::new(&mut asset_bytes, None)?;
///
///     serializer.serialize(&mut ron_serializer)?;
///     Ok(asset_bytes)
/// }
/// ```
///
/// [`try_serialize`]: Self::try_serialize
pub trait ReflectSerializerProcessor {
    /// Attempts to serialize the value which a [`TypedReflectSerializer`] is
    /// currently looking at.
    ///
    /// If you want to override the default deserialization, return
    /// `Ok(Ok(value))` with an `Ok` output from the serializer.
    ///
    /// If you don't want to override the serialization, return ownership of
    /// the serializer back via `Ok(Err(serializer))`.
    ///
    /// To get useful info about the type of value you're serializing, you will
    /// likely want to convert it to a [`Reflect`] and read its type info from
    /// the given registry:
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
    ///         let Some(value) = value.try_as_reflect() else {
    ///             // this value isn't `Reflect`, just do the default serialization
    ///             return Ok(Err(serializer));
    ///         };
    ///         // actually read the type ID of this value
    ///         let type_id = value.reflect_type_info().type_id();
    ///
    ///         if type_id == TypeId::of::<i32>() {
    ///             let value_as_string = format!("{value:?}");
    ///             serializer.serialize_str(&value_as_string).map(Ok)
    ///         } else {
    ///             Ok(Err(serializer))
    ///         }
    ///     }
    /// }
    /// ```
    ///
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

/// A general purpose serializer for reflected types.
///
/// This is the serializer counterpart to [`ReflectDeserializer`].
///
/// See [`TypedReflectSerializer`] for a serializer that serializes a known type.
///
/// # Output
///
/// This serializer will output a map with a single entry,
/// where the key is the _full_ [type path] of the reflected type
/// and the value is the serialized data.
///
/// If you want to override serialization for specific values, you can pass in
/// a reference to a [`ReflectSerializerProcessor`] which will take priority
/// over all other serialization methods - see [`with_processor`].
///
/// # Example
///
/// ```
/// # use bevy_reflect::prelude::*;
/// # use bevy_reflect::{TypeRegistry, serde::ReflectSerializer};
/// #[derive(Reflect, PartialEq, Debug)]
/// #[type_path = "my_crate"]
/// struct MyStruct {
///   value: i32
/// }
///
/// let mut registry = TypeRegistry::default();
/// registry.register::<MyStruct>();
///
/// let input = MyStruct { value: 123 };
///
/// let reflect_serializer = ReflectSerializer::new(&input, &registry);
/// let output = ron::to_string(&reflect_serializer).unwrap();
///
/// assert_eq!(output, r#"{"my_crate::MyStruct":(value:123)}"#);
/// ```
///
/// [`ReflectDeserializer`]: crate::serde::ReflectDeserializer
/// [type path]: crate::TypePath::type_path
/// [`with_processor`]: Self::with_processor
pub struct ReflectSerializer<'a, P = ()> {
    value: &'a dyn PartialReflect,
    registry: &'a TypeRegistry,
    processor: Option<&'a P>,
}

impl<'a> ReflectSerializer<'a, ()> {
    /// Creates a serializer with no processor.
    ///
    /// If you want to add custom logic for serializing certain values, use
    /// [`with_processor`].
    ///
    /// [`with_processor`]: Self::with_processor
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        Self {
            value,
            registry,
            processor: None,
        }
    }
}

impl<'a, P: ReflectSerializerProcessor> ReflectSerializer<'a, P> {
    /// Creates a serializer with a processor.
    ///
    /// If you do not need any custom logic for handling certain values, use
    /// [`new`].
    ///
    /// [`new`]: Self::new
    pub fn with_processor(
        value: &'a dyn PartialReflect,
        registry: &'a TypeRegistry,
        processor: &'a P,
    ) -> Self {
        Self {
            value,
            registry,
            processor: Some(processor),
        }
    }
}

impl<P: ReflectSerializerProcessor> Serialize for ReflectSerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(1))?;
        state.serialize_entry(
            self.value
                .get_represented_type_info()
                .ok_or_else(|| {
                    if self.value.is_dynamic() {
                        make_custom_error(format_args!(
                            "cannot serialize dynamic value without represented type: `{}`",
                            self.value.reflect_type_path()
                        ))
                    } else {
                        make_custom_error(format_args!(
                            "cannot get type info for `{}`",
                            self.value.reflect_type_path()
                        ))
                    }
                })?
                .type_path(),
            &TypedReflectSerializer::new_internal(self.value, self.registry, self.processor),
        )?;
        state.end()
    }
}

/// A serializer for reflected types whose type will be known during deserialization.
///
/// This is the serializer counterpart to [`TypedReflectDeserializer`].
///
/// See [`ReflectSerializer`] for a serializer that serializes an unknown type.
///
/// # Output
///
/// Since the type is expected to be known during deserialization,
/// this serializer will not output any additional type information,
/// such as the [type path].
///
/// Instead, it will output just the serialized data.
///
/// If you want to override serialization for specific values, you can pass in
/// a reference to a [`ReflectSerializerProcessor`] which will take priority
/// over all other serialization methods - see [`with_processor`].
///
/// # Example
///
/// ```
/// # use bevy_reflect::prelude::*;
/// # use bevy_reflect::{TypeRegistry, serde::TypedReflectSerializer};
/// #[derive(Reflect, PartialEq, Debug)]
/// #[type_path = "my_crate"]
/// struct MyStruct {
///   value: i32
/// }
///
/// let mut registry = TypeRegistry::default();
/// registry.register::<MyStruct>();
///
/// let input = MyStruct { value: 123 };
///
/// let reflect_serializer = TypedReflectSerializer::new(&input, &registry);
/// let output = ron::to_string(&reflect_serializer).unwrap();
///
/// assert_eq!(output, r#"(value:123)"#);
/// ```
///
/// [`TypedReflectDeserializer`]: crate::serde::TypedReflectDeserializer
/// [type path]: crate::TypePath::type_path
/// [`with_processor`]: Self::with_processor
pub struct TypedReflectSerializer<'a, P = ()> {
    value: &'a dyn PartialReflect,
    registry: &'a TypeRegistry,
    processor: Option<&'a P>,
}

impl<'a> TypedReflectSerializer<'a, ()> {
    /// Creates a serializer with no processor.
    ///
    /// If you want to add custom logic for serializing certain values, use
    /// [`with_processor`].
    ///
    /// [`with_processor`]: Self::with_processor
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.set(crate::type_info_stack::TypeInfoStack::new());

        Self {
            value,
            registry,
            processor: None,
        }
    }
}

impl<'a, P> TypedReflectSerializer<'a, P> {
    /// Creates a serializer with a processor.
    ///
    /// If you do not need any custom logic for handling certain values, use
    /// [`new`].
    ///
    /// [`new`]: Self::new
    pub fn with_processor(
        value: &'a dyn PartialReflect,
        registry: &'a TypeRegistry,
        processor: &'a P,
    ) -> Self {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.set(crate::type_info_stack::TypeInfoStack::new());

        Self {
            value,
            registry,
            processor: Some(processor),
        }
    }

    /// An internal constructor for creating a serializer without resetting the type info stack.
    pub(super) fn new_internal(
        value: &'a dyn PartialReflect,
        registry: &'a TypeRegistry,
        processor: Option<&'a P>,
    ) -> Self {
        Self {
            value,
            registry,
            processor,
        }
    }
}

impl<P: ReflectSerializerProcessor> Serialize for TypedReflectSerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[cfg(feature = "debug_stack")]
        {
            if let Some(info) = self.value.get_represented_type_info() {
                TYPE_INFO_STACK.with_borrow_mut(|stack| stack.push(info));
            }
        }

        // First, check if our processor wants to serialize this type
        // This takes priority over any other serialization operations
        let serializer = if let Some(processor) = self.processor {
            match processor.try_serialize(self.value, self.registry, serializer) {
                Ok(Ok(value)) => {
                    return Ok(value);
                }
                Err(err) => {
                    return Err(make_custom_error(err));
                }
                Ok(Err(serializer)) => serializer,
            }
        } else {
            serializer
        };

        // Handle both Value case and types that have a custom `Serialize`
        let (serializer, error) = match try_custom_serialize(self.value, self.registry, serializer)
        {
            Ok(result) => return result,
            Err(value) => value,
        };

        let output = match self.value.reflect_ref() {
            ReflectRef::Struct(struct_value) => StructSerializer {
                struct_value,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            ReflectRef::TupleStruct(tuple_struct) => TupleStructSerializer {
                tuple_struct,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            ReflectRef::Tuple(tuple) => TupleSerializer {
                tuple,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            ReflectRef::List(list) => ListSerializer {
                list,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            ReflectRef::Array(array) => ArraySerializer {
                array,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            ReflectRef::Map(map) => MapSerializer {
                map,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            ReflectRef::Set(set) => SetSerializer {
                set,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            ReflectRef::Enum(enum_value) => EnumSerializer {
                enum_value,
                registry: self.registry,
                processor: self.processor,
            }
            .serialize(serializer),
            #[cfg(feature = "functions")]
            ReflectRef::Function(_) => Err(make_custom_error("functions cannot be serialized")),
            ReflectRef::Opaque(_) => Err(error),
        };

        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

        output
    }
}
