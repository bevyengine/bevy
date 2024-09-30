#[cfg(feature = "debug_stack")]
use crate::serde::ser::error_utils::TYPE_INFO_STACK;
use crate::{
    serde::{
        ser::{
            arrays::ArraySerializer, enums::EnumSerializer, error_utils::make_custom_error,
            lists::ListSerializer, maps::MapSerializer, sets::SetSerializer,
            structs::StructSerializer, tuple_structs::TupleStructSerializer,
            tuples::TupleSerializer,
        },
        Serializable,
    },
    PartialReflect, ReflectRef, TypeRegistry,
};
use serde::{ser::SerializeMap, Serialize, Serializer};

pub trait ReflectSerializerProcessor {
    fn try_serialize<S>(
        &self,
        value: &dyn PartialReflect,
        serializer: S,
    ) -> Result<Result<S::Ok, S>, S::Error>
    where
        S: Serializer;
}

impl ReflectSerializerProcessor for () {
    fn try_serialize<S>(
        &self,
        _value: &dyn PartialReflect,
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
pub struct ReflectSerializer<'a, P = ()> {
    value: &'a dyn PartialReflect,
    registry: &'a TypeRegistry,
    processor: Option<&'a P>,
}

impl<'a> ReflectSerializer<'a, ()> {
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        Self {
            value,
            registry,
            processor: None,
        }
    }
}

impl<'a, P: ReflectSerializerProcessor> ReflectSerializer<'a, P> {
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
            &TypedReflectSerializer::with_processor(self.value, self.registry, self.processor),
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
pub struct TypedReflectSerializer<'a, P = ()> {
    value: &'a dyn PartialReflect,
    registry: &'a TypeRegistry,
    processor: Option<&'a P>,
}

impl<'a> TypedReflectSerializer<'a, ()> {
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

        let serializer = if let Some(processor) = self.processor.as_deref() {
            match processor.try_serialize(self.value, serializer) {
                Ok(Ok(value)) => {
                    return Ok(value);
                }
                Err(err) => {
                    return Err(err);
                }
                Ok(Err(serializer)) => serializer,
            }
        } else {
            serializer
        };

        // Handle both Value case and types that have a custom `Serialize`
        let serializable =
            Serializable::try_from_reflect_value::<S::Error>(self.value, self.registry);
        if let Ok(serializable) = serializable {
            #[cfg(feature = "debug_stack")]
            TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

            return serializable.serialize(serializer);
        }

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
            ReflectRef::Opaque(_) => Err(serializable.err().unwrap()),
        };

        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

        output
    }
}
