use crate::serde::ser::arrays::ArraySerializer;
use crate::serde::ser::enums::EnumSerializer;
use crate::serde::ser::error_utils::make_custom_error;
#[cfg(feature = "debug_stack")]
use crate::serde::ser::error_utils::TYPE_INFO_STACK;
use crate::serde::ser::lists::ListSerializer;
use crate::serde::ser::maps::MapSerializer;
use crate::serde::ser::sets::SetSerializer;
use crate::serde::ser::structs::StructSerializer;
use crate::serde::ser::tuple_structs::TupleStructSerializer;
use crate::serde::ser::tuples::TupleSerializer;
use crate::serde::Serializable;
use crate::{PartialReflect, ReflectRef, TypeRegistry};
use serde::ser::SerializeMap;
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
    value: &'a dyn PartialReflect,
    registry: &'a TypeRegistry,
}

impl<'a> ReflectSerializer<'a> {
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        Self { value, registry }
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
    value: &'a dyn PartialReflect,
    registry: &'a TypeRegistry,
}

impl<'a> TypedReflectSerializer<'a> {
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.set(crate::type_info_stack::TypeInfoStack::new());

        Self { value, registry }
    }

    /// An internal constructor for creating a serializer without resetting the type info stack.
    pub(super) fn new_internal(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        Self { value, registry }
    }
}

impl<'a> Serialize for TypedReflectSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[cfg(feature = "debug_stack")]
        {
            if let Some(info) = self.value.get_represented_type_info() {
                TYPE_INFO_STACK.with_borrow_mut(|stack| stack.push(info));
            }
        }

        // Handle both Value case and types that have a custom `Serialize`
        let serializable =
            Serializable::try_from_reflect_value::<S::Error>(self.value, self.registry);
        if let Ok(serializable) = serializable {
            #[cfg(feature = "debug_stack")]
            TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

            return serializable.serialize(serializer);
        }

        let output = match self.value.reflect_ref() {
            ReflectRef::Struct(value) => {
                StructSerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::TupleStruct(value) => {
                TupleStructSerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::Tuple(value) => {
                TupleSerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::List(value) => {
                ListSerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::Array(value) => {
                ArraySerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::Map(value) => {
                MapSerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::Set(value) => {
                SetSerializer::new(value, self.registry).serialize(serializer)
            }
            ReflectRef::Enum(value) => {
                EnumSerializer::new(value, self.registry).serialize(serializer)
            }
            #[cfg(feature = "functions")]
            ReflectRef::Function(_) => Err(make_custom_error("functions cannot be serialized")),
            ReflectRef::Value(_) => Err(serializable.err().unwrap()),
        };

        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

        output
    }
}
