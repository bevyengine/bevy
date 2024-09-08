use crate::serde::ser::structs::StructSerializer;
use crate::serde::{
    ArraySerializer, EnumSerializer, ListSerializer, MapSerializer, Serializable, SetSerializer,
    TupleSerializer, TupleStructSerializer,
};
use crate::{PartialReflect, ReflectRef, TypeRegistry};
use serde::ser::{Error, SerializeMap};
use serde::Serialize;

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
pub struct ReflectSerializer<'a> {
    pub value: &'a dyn PartialReflect,
    pub registry: &'a TypeRegistry,
}

impl<'a> ReflectSerializer<'a> {
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        ReflectSerializer { value, registry }
    }
}

impl<'a> Serialize for ReflectSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(1))?;
        state.serialize_entry(
            self.value
                .get_represented_type_info()
                .ok_or_else(|| {
                    if self.value.is_dynamic() {
                        Error::custom(format_args!(
                            "cannot serialize dynamic value without represented type: {}",
                            self.value.reflect_type_path()
                        ))
                    } else {
                        Error::custom(format_args!(
                            "cannot get type info for {}",
                            self.value.reflect_type_path()
                        ))
                    }
                })?
                .type_path(),
            &TypedReflectSerializer::new(self.value, self.registry),
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
pub struct TypedReflectSerializer<'a> {
    pub value: &'a dyn PartialReflect,
    pub registry: &'a TypeRegistry,
}

impl<'a> TypedReflectSerializer<'a> {
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        TypedReflectSerializer { value, registry }
    }
}

impl<'a> Serialize for TypedReflectSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Handle both Value case and types that have a custom `Serialize`
        let serializable =
            Serializable::try_from_reflect_value::<S::Error>(self.value, self.registry);
        if let Ok(serializable) = serializable {
            return serializable.serialize(serializer);
        }

        match self.value.reflect_ref() {
            ReflectRef::Struct(value) => {
                StructSerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::TupleStruct(value) => TupleStructSerializer {
                tuple_struct: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Tuple(value) => TupleSerializer {
                tuple: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::List(value) => ListSerializer {
                list: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Array(value) => ArraySerializer {
                array: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Map(value) => MapSerializer {
                map: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Set(value) => SetSerializer {
                set: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Enum(value) => EnumSerializer {
                enum_value: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Value(_) => Err(serializable.err().unwrap()),
        }
    }
}
