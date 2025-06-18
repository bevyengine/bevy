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

use super::ReflectSerializerProcessor;

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
