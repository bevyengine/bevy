#[cfg(feature = "debug_stack")]
use crate::serde::de::error_utils::TYPE_INFO_STACK;
use crate::serde::{ReflectDeserializeWithRegistry, SerializationData};
use crate::{
    serde::{
        de::{
            arrays::ArrayVisitor, enums::EnumVisitor, error_utils::make_custom_error,
            lists::ListVisitor, maps::MapVisitor, options::OptionVisitor, sets::SetVisitor,
            structs::StructVisitor, tuple_structs::TupleStructVisitor, tuples::TupleVisitor,
        },
        TypeRegistrationDeserializer,
    },
    PartialReflect, ReflectDeserialize, TypeInfo, TypePath, TypeRegistration, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{DeserializeSeed, Error, IgnoredAny, MapAccess, Visitor};

/// Allows overriding the default deserialization behaviour of
/// [`ReflectDeserializer`] and [`TypedReflectDeserializer`] for specific
/// [`TypeRegistration`]s.
///
/// When deserializing a reflected value, you may want to override the default
/// behaviour and use your own logic for deserialization. This logic may also
/// be context-dependent, and only apply for a single use of your
/// [`ReflectDeserializer`]. To achieve this, you can create a processor and
/// pass it in to your deserializer.
///
/// Whenever the deserializer attempts to deserialize a value, it will first
/// call [`try_deserialize`] on your processor, which may take ownership of the
/// deserializer and give back a [`Box<dyn PartialReflect>`], or return
/// ownership of the deserializer back, and continue with the default logic.
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
/// [`try_deserialize`]: Self::try_deserialize
pub trait ReflectDeserializerProcessor {
    /// Attempts to deserialize the value which a [`TypedReflectDeserializer`]
    /// is currently looking at, and knows the type of.
    ///
    /// If you've read the `registration` and want to override the default
    /// deserialization, return `Ok(Ok(value))` with the boxed reflected value
    /// that you want to assign this value to. The type inside the box must
    /// be the same one as the `registration` is for.
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
/// For opaque types (i.e. [`ReflectKind::Opaque`]) or types that register [`ReflectDeserialize`] type data,
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
/// If you want to override deserialization for a specific [`TypeRegistration`],
/// you can pass in a reference to a [`ReflectDeserializerProcessor`] which will
/// take priority over all other deserialization methods - see [`with_processor`].
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
/// // Since `MyStruct` is not an opaque type and does not register `ReflectDeserialize`,
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
/// [`ReflectKind::Opaque`]: crate::ReflectKind::Opaque
/// [`ReflectDeserialize`]: crate::ReflectDeserialize
/// [`Box<DynamicStruct>`]: crate::DynamicStruct
/// [`Box<DynamicList>`]: crate::DynamicList
/// [`FromReflect`]: crate::FromReflect
/// [`ReflectFromReflect`]: crate::ReflectFromReflect
/// [`with_processor`]: Self::with_processor
pub struct ReflectDeserializer<'a, P = ()> {
    registry: &'a TypeRegistry,
    processor: Option<&'a mut P>,
}

impl<'a> ReflectDeserializer<'a, ()> {
    /// Creates a deserializer with no processor.
    ///
    /// If you want to add custom logic for deserializing certain types, use
    /// [`with_processor`].
    ///
    /// [`with_processor`]: Self::with_processor
    pub fn new(registry: &'a TypeRegistry) -> Self {
        Self {
            registry,
            processor: None,
        }
    }
}

impl<'a, P: ReflectDeserializerProcessor> ReflectDeserializer<'a, P> {
    /// Creates a deserializer with a processor.
    ///
    /// If you do not need any custom logic for handling certain types, use
    /// [`new`].
    ///
    /// [`new`]: Self::new
    pub fn with_processor(registry: &'a TypeRegistry, processor: &'a mut P) -> Self {
        Self {
            registry,
            processor: Some(processor),
        }
    }
}

impl<'de, P: ReflectDeserializerProcessor> DeserializeSeed<'de> for ReflectDeserializer<'_, P> {
    type Value = Box<dyn PartialReflect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UntypedReflectDeserializerVisitor<'a, P> {
            registry: &'a TypeRegistry,
            processor: Option<&'a mut P>,
        }

        impl<'de, P: ReflectDeserializerProcessor> Visitor<'de>
            for UntypedReflectDeserializerVisitor<'_, P>
        {
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

                let value = map.next_value_seed(TypedReflectDeserializer::new_internal(
                    registration,
                    self.registry,
                    self.processor,
                ))?;

                if map.next_key::<IgnoredAny>()?.is_some() {
                    return Err(Error::invalid_length(2, &"a single entry"));
                }

                Ok(value)
            }
        }

        deserializer.deserialize_map(UntypedReflectDeserializerVisitor {
            registry: self.registry,
            processor: self.processor,
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
/// For opaque types (i.e. [`ReflectKind::Opaque`]) or types that register [`ReflectDeserialize`] type data,
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
/// If you want to override deserialization for a specific [`TypeRegistration`],
/// you can pass in a reference to a [`ReflectDeserializerProcessor`] which will
/// take priority over all other deserialization methods - see [`with_processor`].
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
/// // Since `MyStruct` is not an opaque type and does not register `ReflectDeserialize`,
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
/// [`ReflectKind::Opaque`]: crate::ReflectKind::Opaque
/// [`ReflectDeserialize`]: crate::ReflectDeserialize
/// [`Box<DynamicStruct>`]: crate::DynamicStruct
/// [`Box<DynamicList>`]: crate::DynamicList
/// [`FromReflect`]: crate::FromReflect
/// [`ReflectFromReflect`]: crate::ReflectFromReflect
/// [`with_processor`]: Self::with_processor
pub struct TypedReflectDeserializer<'a, P = ()> {
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
    processor: Option<&'a mut P>,
}

impl<'a> TypedReflectDeserializer<'a, ()> {
    /// Creates a typed deserializer with no processor.
    ///
    /// If you want to add custom logic for deserializing certain types, use
    /// [`with_processor`].
    ///
    /// [`with_processor`]: Self::with_processor
    pub fn new(registration: &'a TypeRegistration, registry: &'a TypeRegistry) -> Self {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.set(crate::type_info_stack::TypeInfoStack::new());

        Self {
            registration,
            registry,
            processor: None,
        }
    }

    /// Creates a new [`TypedReflectDeserializer`] for the given type `T`
    /// without a processor.
    ///
    /// # Panics
    ///
    /// Panics if `T` is not registered in the given [`TypeRegistry`].
    pub fn of<T: TypePath>(registry: &'a TypeRegistry) -> Self {
        let registration = registry
            .get(core::any::TypeId::of::<T>())
            .unwrap_or_else(|| panic!("no registration found for type `{}`", T::type_path()));

        Self {
            registration,
            registry,
            processor: None,
        }
    }
}

impl<'a, P: ReflectDeserializerProcessor> TypedReflectDeserializer<'a, P> {
    /// Creates a typed deserializer with a processor.
    ///
    /// If you do not need any custom logic for handling certain types, use
    /// [`new`].
    ///
    /// [`new`]: Self::new
    pub fn with_processor(
        registration: &'a TypeRegistration,
        registry: &'a TypeRegistry,
        processor: &'a mut P,
    ) -> Self {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.set(crate::type_info_stack::TypeInfoStack::new());

        Self {
            registration,
            registry,
            processor: Some(processor),
        }
    }

    /// An internal constructor for creating a deserializer without resetting the type info stack.
    pub(super) fn new_internal(
        registration: &'a TypeRegistration,
        registry: &'a TypeRegistry,
        processor: Option<&'a mut P>,
    ) -> Self {
        Self {
            registration,
            registry,
            processor,
        }
    }
}

impl<'de, P: ReflectDeserializerProcessor> DeserializeSeed<'de>
    for TypedReflectDeserializer<'_, P>
{
    type Value = Box<dyn PartialReflect>;

    fn deserialize<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialize_internal = || -> Result<Self::Value, D::Error> {
            // First, check if our processor wants to deserialize this type
            // This takes priority over any other deserialization operations
            let deserializer = if let Some(processor) = self.processor.as_deref_mut() {
                match processor.try_deserialize(self.registration, self.registry, deserializer) {
                    Ok(Ok(value)) => {
                        return Ok(value);
                    }
                    Err(err) => {
                        return Err(make_custom_error(err));
                    }
                    Ok(Err(deserializer)) => deserializer,
                }
            } else {
                deserializer
            };

            let type_path = self.registration.type_info().type_path();

            // Handle both Value case and types that have a custom `ReflectDeserialize`
            if let Some(deserialize_reflect) = self.registration.data::<ReflectDeserialize>() {
                let value = deserialize_reflect.deserialize(deserializer)?;
                return Ok(value.into_partial_reflect());
            }

            if let Some(deserialize_reflect) =
                self.registration.data::<ReflectDeserializeWithRegistry>()
            {
                let value = deserialize_reflect.deserialize(deserializer, self.registry)?;
                return Ok(value);
            }

            match self.registration.type_info() {
                TypeInfo::Struct(struct_info) => {
                    let mut dynamic_struct = deserializer.deserialize_struct(
                        struct_info.type_path_table().ident().unwrap(),
                        struct_info.field_names(),
                        StructVisitor {
                            struct_info,
                            registration: self.registration,
                            registry: self.registry,
                            processor: self.processor,
                        },
                    )?;
                    dynamic_struct.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_struct))
                }
                TypeInfo::TupleStruct(tuple_struct_info) => {
                    let mut dynamic_tuple_struct = if tuple_struct_info.field_len() == 1
                        && self.registration.data::<SerializationData>().is_none()
                    {
                        deserializer.deserialize_newtype_struct(
                            tuple_struct_info.type_path_table().ident().unwrap(),
                            TupleStructVisitor {
                                tuple_struct_info,
                                registration: self.registration,
                                registry: self.registry,
                                processor: self.processor,
                            },
                        )?
                    } else {
                        deserializer.deserialize_tuple_struct(
                            tuple_struct_info.type_path_table().ident().unwrap(),
                            tuple_struct_info.field_len(),
                            TupleStructVisitor {
                                tuple_struct_info,
                                registration: self.registration,
                                registry: self.registry,
                                processor: self.processor,
                            },
                        )?
                    };
                    dynamic_tuple_struct.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_tuple_struct))
                }
                TypeInfo::List(list_info) => {
                    let mut dynamic_list = deserializer.deserialize_seq(ListVisitor {
                        list_info,
                        registry: self.registry,
                        processor: self.processor,
                    })?;
                    dynamic_list.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_list))
                }
                TypeInfo::Array(array_info) => {
                    let mut dynamic_array = deserializer.deserialize_tuple(
                        array_info.capacity(),
                        ArrayVisitor {
                            array_info,
                            registry: self.registry,
                            processor: self.processor,
                        },
                    )?;
                    dynamic_array.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_array))
                }
                TypeInfo::Map(map_info) => {
                    let mut dynamic_map = deserializer.deserialize_map(MapVisitor {
                        map_info,
                        registry: self.registry,
                        processor: self.processor,
                    })?;
                    dynamic_map.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_map))
                }
                TypeInfo::Set(set_info) => {
                    let mut dynamic_set = deserializer.deserialize_seq(SetVisitor {
                        set_info,
                        registry: self.registry,
                        processor: self.processor,
                    })?;
                    dynamic_set.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_set))
                }
                TypeInfo::Tuple(tuple_info) => {
                    let mut dynamic_tuple = deserializer.deserialize_tuple(
                        tuple_info.field_len(),
                        TupleVisitor {
                            tuple_info,
                            registration: self.registration,
                            registry: self.registry,
                            processor: self.processor,
                        },
                    )?;
                    dynamic_tuple.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_tuple))
                }
                TypeInfo::Enum(enum_info) => {
                    let mut dynamic_enum = if enum_info.type_path_table().module_path()
                        == Some("core::option")
                        && enum_info.type_path_table().ident() == Some("Option")
                    {
                        deserializer.deserialize_option(OptionVisitor {
                            enum_info,
                            registry: self.registry,
                            processor: self.processor,
                        })?
                    } else {
                        deserializer.deserialize_enum(
                            enum_info.type_path_table().ident().unwrap(),
                            enum_info.variant_names(),
                            EnumVisitor {
                                enum_info,
                                registration: self.registration,
                                registry: self.registry,
                                processor: self.processor,
                            },
                        )?
                    };
                    dynamic_enum.set_represented_type(Some(self.registration.type_info()));
                    Ok(Box::new(dynamic_enum))
                }
                TypeInfo::Opaque(_) => {
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
