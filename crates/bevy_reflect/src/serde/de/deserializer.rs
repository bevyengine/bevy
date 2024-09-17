use crate::serde::de::arrays::ArrayVisitor;
use crate::serde::de::enums::EnumVisitor;
use crate::serde::de::error_utils::make_custom_error;
#[cfg(feature = "debug_stack")]
use crate::serde::de::error_utils::TYPE_INFO_STACK;
use crate::serde::de::lists::ListVisitor;
use crate::serde::de::maps::MapVisitor;
use crate::serde::de::options::OptionVisitor;
use crate::serde::de::sets::SetVisitor;
use crate::serde::de::structs::StructVisitor;
use crate::serde::de::tuple_structs::TupleStructVisitor;
use crate::serde::de::tuples::TupleVisitor;
use crate::serde::TypeRegistrationDeserializer;
use crate::{PartialReflect, ReflectDeserialize, TypeInfo, TypeRegistration, TypeRegistry};
use core::fmt::Formatter;
use serde::de::{DeserializeSeed, Error, IgnoredAny, MapAccess, Visitor};
use std::fmt;

/// A general purpose deserializer for reflected types.
///
/// This is the deserializer counterpart to [`ReflectSerializer`].
///
/// See [`TypedReflectDeserializer`] for a deserializer that expects a known type.
///
/// # Input
///
/// This deserializer expects a map with a single entry,
/// where the key is the _full_ [type path] of the reflected type
/// and the value is the serialized data.
///
/// # Output
///
/// This deserializer will return a [`Box<dyn Reflect>`] containing the deserialized data.
///
/// For value types (i.e. [`ReflectKind::Value`]) or types that register [`ReflectDeserialize`] type data,
/// this `Box` will contain the expected type.
/// For example, deserializing an `i32` will return a `Box<i32>` (as a `Box<dyn Reflect>`).
///
/// Otherwise, this `Box` will contain the dynamic equivalent.
/// For example, a deserialized struct might return a [`Box<DynamicStruct>`]
/// and a deserialized `Vec` might return a [`Box<DynamicList>`].
///
/// This means that if the actual type is needed, these dynamic representations will need to
/// be converted to the concrete type using [`FromReflect`] or [`ReflectFromReflect`].
///
/// # Example
///
/// ```
/// # use serde::de::DeserializeSeed;
/// # use bevy_reflect::prelude::*;
/// # use bevy_reflect::{DynamicStruct, TypeRegistry, serde::ReflectDeserializer};
/// #[derive(Reflect, PartialEq, Debug)]
/// #[type_path = "my_crate"]
/// struct MyStruct {
///   value: i32
/// }
///
/// let mut registry = TypeRegistry::default();
/// registry.register::<MyStruct>();
///
/// let input = r#"{
///   "my_crate::MyStruct": (
///     value: 123
///   )
/// }"#;
///
/// let mut deserializer = ron::Deserializer::from_str(input).unwrap();
/// let reflect_deserializer = ReflectDeserializer::new(&registry);
///
/// let output: Box<dyn PartialReflect> = reflect_deserializer.deserialize(&mut deserializer).unwrap();
///
/// // Since `MyStruct` is not a value type and does not register `ReflectDeserialize`,
/// // we know that its deserialized value will be a `DynamicStruct`,
/// // although it will represent `MyStruct`.
/// assert!(output.as_partial_reflect().represents::<MyStruct>());
///
/// // We can convert back to `MyStruct` using `FromReflect`.
/// let value: MyStruct = <MyStruct as FromReflect>::from_reflect(output.as_partial_reflect()).unwrap();
/// assert_eq!(value, MyStruct { value: 123 });
///
/// // We can also do this dynamically with `ReflectFromReflect`.
/// let type_id = output.get_represented_type_info().unwrap().type_id();
/// let reflect_from_reflect = registry.get_type_data::<ReflectFromReflect>(type_id).unwrap();
/// let value: Box<dyn Reflect> = reflect_from_reflect.from_reflect(output.as_partial_reflect()).unwrap();
/// assert!(value.is::<MyStruct>());
/// assert_eq!(value.take::<MyStruct>().unwrap(), MyStruct { value: 123 });
/// ```
///
/// [`ReflectSerializer`]: crate::serde::ReflectSerializer
/// [type path]: crate::TypePath::type_path
/// [`Box<dyn Reflect>`]: crate::Reflect
/// [`ReflectKind::Value`]: crate::ReflectKind::Value
/// [`ReflectDeserialize`]: crate::ReflectDeserialize
/// [`Box<DynamicStruct>`]: crate::DynamicStruct
/// [`Box<DynamicList>`]: crate::DynamicList
/// [`FromReflect`]: crate::FromReflect
/// [`ReflectFromReflect`]: crate::ReflectFromReflect
pub struct ReflectDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a> ReflectDeserializer<'a> {
    pub fn new(registry: &'a TypeRegistry) -> Self {
        Self { registry }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for ReflectDeserializer<'a> {
    type Value = Box<dyn PartialReflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UntypedReflectDeserializerVisitor<'a> {
            registry: &'a TypeRegistry,
        }

        impl<'a, 'de> Visitor<'de> for UntypedReflectDeserializerVisitor<'a> {
            type Value = Box<dyn PartialReflect>;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter
                    .write_str("map containing `type` and `value` entries for the reflected value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let registration = map
                    .next_key_seed(TypeRegistrationDeserializer::new(self.registry))?
                    .ok_or_else(|| Error::invalid_length(0, &"a single entry"))?;

                let value = map.next_value_seed(TypedReflectDeserializer {
                    registration,
                    registry: self.registry,
                })?;

                if map.next_key::<IgnoredAny>()?.is_some() {
                    return Err(Error::invalid_length(2, &"a single entry"));
                }

                Ok(value)
            }
        }

        deserializer.deserialize_map(UntypedReflectDeserializerVisitor {
            registry: self.registry,
        })
    }
}

/// A deserializer for reflected types whose [`TypeRegistration`] is known.
///
/// This is the deserializer counterpart to [`TypedReflectSerializer`].
///
/// See [`ReflectDeserializer`] for a deserializer that expects an unknown type.
///
/// # Input
///
/// Since the type is already known, the input is just the serialized data.
///
/// # Output
///
/// This deserializer will return a [`Box<dyn Reflect>`] containing the deserialized data.
///
/// For value types (i.e. [`ReflectKind::Value`]) or types that register [`ReflectDeserialize`] type data,
/// this `Box` will contain the expected type.
/// For example, deserializing an `i32` will return a `Box<i32>` (as a `Box<dyn Reflect>`).
///
/// Otherwise, this `Box` will contain the dynamic equivalent.
/// For example, a deserialized struct might return a [`Box<DynamicStruct>`]
/// and a deserialized `Vec` might return a [`Box<DynamicList>`].
///
/// This means that if the actual type is needed, these dynamic representations will need to
/// be converted to the concrete type using [`FromReflect`] or [`ReflectFromReflect`].
///
/// # Example
///
/// ```
/// # use std::any::TypeId;
/// # use serde::de::DeserializeSeed;
/// # use bevy_reflect::prelude::*;
/// # use bevy_reflect::{DynamicStruct, TypeRegistry, serde::TypedReflectDeserializer};
/// #[derive(Reflect, PartialEq, Debug)]
/// struct MyStruct {
///   value: i32
/// }
///
/// let mut registry = TypeRegistry::default();
/// registry.register::<MyStruct>();
///
/// let input = r#"(
///   value: 123
/// )"#;
///
/// let registration = registry.get(TypeId::of::<MyStruct>()).unwrap();
///
/// let mut deserializer = ron::Deserializer::from_str(input).unwrap();
/// let reflect_deserializer = TypedReflectDeserializer::new(registration, &registry);
///
/// let output: Box<dyn PartialReflect> = reflect_deserializer.deserialize(&mut deserializer).unwrap();
///
/// // Since `MyStruct` is not a value type and does not register `ReflectDeserialize`,
/// // we know that its deserialized value will be a `DynamicStruct`,
/// // although it will represent `MyStruct`.
/// assert!(output.as_partial_reflect().represents::<MyStruct>());
///
/// // We can convert back to `MyStruct` using `FromReflect`.
/// let value: MyStruct = <MyStruct as FromReflect>::from_reflect(output.as_partial_reflect()).unwrap();
/// assert_eq!(value, MyStruct { value: 123 });
///
/// // We can also do this dynamically with `ReflectFromReflect`.
/// let type_id = output.get_represented_type_info().unwrap().type_id();
/// let reflect_from_reflect = registry.get_type_data::<ReflectFromReflect>(type_id).unwrap();
/// let value: Box<dyn Reflect> = reflect_from_reflect.from_reflect(output.as_partial_reflect()).unwrap();
/// assert!(value.is::<MyStruct>());
/// assert_eq!(value.take::<MyStruct>().unwrap(), MyStruct { value: 123 });
/// ```
///
/// [`TypedReflectSerializer`]: crate::serde::TypedReflectSerializer
/// [`Box<dyn Reflect>`]: crate::Reflect
/// [`ReflectKind::Value`]: crate::ReflectKind::Value
/// [`ReflectDeserialize`]: crate::ReflectDeserialize
/// [`Box<DynamicStruct>`]: crate::DynamicStruct
/// [`Box<DynamicList>`]: crate::DynamicList
/// [`FromReflect`]: crate::FromReflect
/// [`ReflectFromReflect`]: crate::ReflectFromReflect
pub struct TypedReflectDeserializer<'a> {
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a> TypedReflectDeserializer<'a> {
    pub fn new(registration: &'a TypeRegistration, registry: &'a TypeRegistry) -> Self {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.set(crate::type_info_stack::TypeInfoStack::new());

        Self {
            registration,
            registry,
        }
    }

    /// An internal constructor for creating a deserializer without resetting the type info stack.
    pub(super) fn new_internal(
        registration: &'a TypeRegistration,
        registry: &'a TypeRegistry,
    ) -> Self {
        Self {
            registration,
            registry,
        }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for TypedReflectDeserializer<'a> {
    type Value = Box<dyn PartialReflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialize_internal = || -> Result<Self::Value, D::Error> {
            let type_path = self.registration.type_info().type_path();

            // Handle both Value case and types that have a custom `ReflectDeserialize`
            if let Some(deserialize_reflect) = self.registration.data::<ReflectDeserialize>() {
                let value = deserialize_reflect.deserialize(deserializer)?;
                return Ok(value.into_partial_reflect());
            }

            match self.registration.type_info() {
                TypeInfo::Struct(struct_info) => {
                    let mut dynamic_struct = deserializer.deserialize_struct(
                        struct_info.type_path_table().ident().unwrap(),
                        struct_info.field_names(),
                        StructVisitor::new(struct_info, self.registration, self.registry),
                    )?;
                    dynamic_struct.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_struct))
                }
                TypeInfo::TupleStruct(tuple_struct_info) => {
                    let mut dynamic_tuple_struct = deserializer.deserialize_tuple_struct(
                        tuple_struct_info.type_path_table().ident().unwrap(),
                        tuple_struct_info.field_len(),
                        TupleStructVisitor::new(
                            tuple_struct_info,
                            self.registration,
                            self.registry,
                        ),
                    )?;
                    dynamic_tuple_struct.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_tuple_struct))
                }
                TypeInfo::List(list_info) => {
                    let mut dynamic_list =
                        deserializer.deserialize_seq(ListVisitor::new(list_info, self.registry))?;
                    dynamic_list.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_list))
                }
                TypeInfo::Array(array_info) => {
                    let mut dynamic_array = deserializer.deserialize_tuple(
                        array_info.capacity(),
                        ArrayVisitor::new(array_info, self.registry),
                    )?;
                    dynamic_array.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_array))
                }
                TypeInfo::Map(map_info) => {
                    let mut dynamic_map =
                        deserializer.deserialize_map(MapVisitor::new(map_info, self.registry))?;
                    dynamic_map.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_map))
                }
                TypeInfo::Set(set_info) => {
                    let mut dynamic_set =
                        deserializer.deserialize_seq(SetVisitor::new(set_info, self.registry))?;
                    dynamic_set.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_set))
                }
                TypeInfo::Tuple(tuple_info) => {
                    let mut dynamic_tuple = deserializer.deserialize_tuple(
                        tuple_info.field_len(),
                        TupleVisitor::new(tuple_info, self.registration, self.registry),
                    )?;
                    dynamic_tuple.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_tuple))
                }
                TypeInfo::Enum(enum_info) => {
                    let mut dynamic_enum = if enum_info.type_path_table().module_path()
                        == Some("core::option")
                        && enum_info.type_path_table().ident() == Some("Option")
                    {
                        deserializer
                            .deserialize_option(OptionVisitor::new(enum_info, self.registry))?
                    } else {
                        deserializer.deserialize_enum(
                            enum_info.type_path_table().ident().unwrap(),
                            enum_info.variant_names(),
                            EnumVisitor::new(enum_info, self.registration, self.registry),
                        )?
                    };
                    dynamic_enum.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_enum))
                }
                TypeInfo::Value(_) => {
                    // This case should already be handled
                    Err(make_custom_error(format_args!(
                        "type `{type_path}` did not register the `ReflectDeserialize` type data. For certain types, this may need to be registered manually using `register_type_data`",
                    )))
                }
            }
        };

        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.with_borrow_mut(|stack| stack.push(self.registration.type_info()));

        let output = deserialize_internal();

        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

        output
    }
}
