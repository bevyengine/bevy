pub use deserializer::*;
pub use registrations::*;

mod deserializer;
mod helpers;
mod registration_utils;
mod registrations;
mod struct_utils;
mod tuple_utils;

use crate::serde::de::helpers::ExpectedValues;
use crate::serde::de::registration_utils::{try_get_registration, GetFieldRegistration};
use crate::serde::de::struct_utils::{visit_struct, visit_struct_seq};
use crate::serde::de::tuple_utils::visit_tuple;
use crate::{
    ArrayInfo, DynamicArray, DynamicEnum, DynamicList, DynamicMap, DynamicSet, DynamicStruct,
    DynamicTuple, DynamicTupleStruct, DynamicVariant, EnumInfo, ListInfo, Map, MapInfo, Reflect,
    Set, SetInfo, StructInfo, StructVariantInfo, TupleInfo, TupleStructInfo, TupleVariantInfo,
    TypeRegistration, TypeRegistry, VariantInfo,
};
use erased_serde::Deserializer;
use serde::de::{DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, VariantAccess, Visitor};
use std::fmt;
use std::fmt::Formatter;

pub trait DeserializeValue {
    fn deserialize(
        deserializer: &mut dyn Deserializer,
        type_registry: &TypeRegistry,
    ) -> Result<Box<dyn Reflect>, erased_serde::Error>;
}

struct StructVisitor<'a> {
    struct_info: &'static StructInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for StructVisitor<'a> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected struct value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        visit_struct_seq(&mut seq, self.struct_info, self.registration, self.registry)
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_struct(&mut map, self.struct_info, self.registration, self.registry)
    }
}

struct TupleStructVisitor<'a> {
    tuple_struct_info: &'static TupleStructInfo,
    registry: &'a TypeRegistry,
    registration: &'a TypeRegistration,
}

impl<'a, 'de> Visitor<'de> for TupleStructVisitor<'a> {
    type Value = DynamicTupleStruct;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected tuple struct value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(
            &mut seq,
            self.tuple_struct_info,
            self.registration,
            self.registry,
        )
        .map(DynamicTupleStruct::from)
    }
}

struct TupleVisitor<'a> {
    tuple_info: &'static TupleInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleVisitor<'a> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected tuple value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(&mut seq, self.tuple_info, self.registration, self.registry)
    }
}

struct ArrayVisitor<'a> {
    array_info: &'static ArrayInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ArrayVisitor<'a> {
    type Value = DynamicArray;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected array value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        let registration = try_get_registration(self.array_info.item_ty(), self.registry)?;
        while let Some(value) =
            seq.next_element_seed(TypedReflectDeserializer::new(registration, self.registry))?
        {
            vec.push(value);
        }

        if vec.len() != self.array_info.capacity() {
            return Err(Error::invalid_length(
                vec.len(),
                &self.array_info.capacity().to_string().as_str(),
            ));
        }

        Ok(DynamicArray::new(vec.into_boxed_slice()))
    }
}

struct ListVisitor<'a> {
    list_info: &'static ListInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ListVisitor<'a> {
    type Value = DynamicList;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected list value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut list = DynamicList::default();
        let registration = try_get_registration(self.list_info.item_ty(), self.registry)?;
        while let Some(value) =
            seq.next_element_seed(TypedReflectDeserializer::new(registration, self.registry))?
        {
            list.push_box(value);
        }
        Ok(list)
    }
}

struct MapVisitor<'a> {
    map_info: &'static MapInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for MapVisitor<'a> {
    type Value = DynamicMap;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected map value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut dynamic_map = DynamicMap::default();
        let key_registration = try_get_registration(self.map_info.key_ty(), self.registry)?;
        let value_registration = try_get_registration(self.map_info.value_ty(), self.registry)?;
        while let Some(key) = map.next_key_seed(TypedReflectDeserializer::new(
            key_registration,
            self.registry,
        ))? {
            let value = map.next_value_seed(TypedReflectDeserializer::new(
                value_registration,
                self.registry,
            ))?;
            dynamic_map.insert_boxed(key, value);
        }

        Ok(dynamic_map)
    }
}

struct SetVisitor<'a> {
    set_info: &'static SetInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SetVisitor<'a> {
    type Value = DynamicSet;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected set value")
    }

    fn visit_seq<V>(self, mut set: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut dynamic_set = DynamicSet::default();
        let value_registration = try_get_registration(self.set_info.value_ty(), self.registry)?;
        while let Some(value) = set.next_element_seed(TypedReflectDeserializer::new(
            value_registration,
            self.registry,
        ))? {
            dynamic_set.insert_boxed(value);
        }

        Ok(dynamic_set)
    }
}

struct EnumVisitor<'a> {
    enum_info: &'static EnumInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for EnumVisitor<'a> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected enum value")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let mut dynamic_enum = DynamicEnum::default();
        let (variant_info, variant) = data.variant_seed(VariantDeserializer {
            enum_info: self.enum_info,
        })?;

        let value: DynamicVariant = match variant_info {
            VariantInfo::Unit(..) => variant.unit_variant()?.into(),
            VariantInfo::Struct(struct_info) => variant
                .struct_variant(
                    struct_info.field_names(),
                    StructVariantVisitor {
                        struct_info,
                        registration: self.registration,
                        registry: self.registry,
                    },
                )?
                .into(),
            VariantInfo::Tuple(tuple_info) if tuple_info.field_len() == 1 => {
                let registration = tuple_info.get_field_registration(0, self.registry)?;
                let value = variant.newtype_variant_seed(TypedReflectDeserializer::new(
                    registration,
                    self.registry,
                ))?;
                let mut dynamic_tuple = DynamicTuple::default();
                dynamic_tuple.insert_boxed(value);
                dynamic_tuple.into()
            }
            VariantInfo::Tuple(tuple_info) => variant
                .tuple_variant(
                    tuple_info.field_len(),
                    TupleVariantVisitor {
                        tuple_info,
                        registration: self.registration,
                        registry: self.registry,
                    },
                )?
                .into(),
        };
        let variant_name = variant_info.name();
        let variant_index = self
            .enum_info
            .index_of(variant_name)
            .expect("variant should exist");
        dynamic_enum.set_variant_with_index(variant_index, variant_name, value);
        Ok(dynamic_enum)
    }
}

struct VariantDeserializer {
    enum_info: &'static EnumInfo,
}

impl<'de> DeserializeSeed<'de> for VariantDeserializer {
    type Value = &'static VariantInfo;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VariantVisitor(&'static EnumInfo);

        impl<'de> Visitor<'de> for VariantVisitor {
            type Value = &'static VariantInfo;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("expected either a variant index or variant name")
            }

            fn visit_u32<E>(self, variant_index: u32) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.0.variant_at(variant_index as usize).ok_or_else(|| {
                    Error::custom(format_args!(
                        "no variant found at index `{}` on enum `{}`",
                        variant_index,
                        self.0.type_path()
                    ))
                })
            }

            fn visit_str<E>(self, variant_name: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.0.variant(variant_name).ok_or_else(|| {
                    let names = self.0.iter().map(VariantInfo::name);
                    Error::custom(format_args!(
                        "unknown variant `{}`, expected one of {:?}",
                        variant_name,
                        ExpectedValues::from_iter(names)
                    ))
                })
            }
        }

        deserializer.deserialize_identifier(VariantVisitor(self.enum_info))
    }
}

struct StructVariantVisitor<'a> {
    struct_info: &'static StructVariantInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for StructVariantVisitor<'a> {
    type Value = DynamicStruct;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected struct variant value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        visit_struct_seq(&mut seq, self.struct_info, self.registration, self.registry)
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_struct(&mut map, self.struct_info, self.registration, self.registry)
    }
}

struct TupleVariantVisitor<'a> {
    tuple_info: &'static TupleVariantInfo,
    registration: &'a TypeRegistration,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for TupleVariantVisitor<'a> {
    type Value = DynamicTuple;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected tuple variant value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        visit_tuple(&mut seq, self.tuple_info, self.registration, self.registry)
    }
}

struct OptionVisitor<'a> {
    enum_info: &'static EnumInfo,
    registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for OptionVisitor<'a> {
    type Value = DynamicEnum;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected option value of type ")?;
        formatter.write_str(self.enum_info.type_path())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut option = DynamicEnum::default();
        option.set_variant("None", ());
        Ok(option)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let variant_info = self.enum_info.variant("Some").unwrap();
        match variant_info {
            VariantInfo::Tuple(tuple_info) if tuple_info.field_len() == 1 => {
                let field = tuple_info.field_at(0).unwrap();
                let registration = try_get_registration(*field.ty(), self.registry)?;
                let de = TypedReflectDeserializer::new(registration, self.registry);
                let mut value = DynamicTuple::default();
                value.insert_boxed(de.deserialize(deserializer)?);
                let mut option = DynamicEnum::default();
                option.set_variant("Some", value);
                Ok(option)
            }
            info => Err(Error::custom(format_args!(
                "invalid variant, expected `Some` but got `{}`",
                info.name()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use bincode::Options;
    use std::any::TypeId;
    use std::f32::consts::PI;
    use std::ops::RangeInclusive;

    use serde::de::DeserializeSeed;
    use serde::Deserialize;

    use bevy_utils::{HashMap, HashSet};

    use crate as bevy_reflect;
    use crate::serde::{ReflectDeserializer, ReflectSerializer, TypedReflectDeserializer};
    use crate::{
        DynamicEnum, FromReflect, PartialReflect, Reflect, ReflectDeserialize, TypeRegistry,
    };

    #[derive(Reflect, Debug, PartialEq)]
    struct MyStruct {
        primitive_value: i8,
        option_value: Option<String>,
        option_value_complex: Option<SomeStruct>,
        tuple_value: (f32, usize),
        list_value: Vec<i32>,
        array_value: [i32; 5],
        map_value: HashMap<u8, usize>,
        set_value: HashSet<u8>,
        struct_value: SomeStruct,
        tuple_struct_value: SomeTupleStruct,
        unit_struct: SomeUnitStruct,
        unit_enum: SomeEnum,
        newtype_enum: SomeEnum,
        tuple_enum: SomeEnum,
        struct_enum: SomeEnum,
        ignored_struct: SomeIgnoredStruct,
        ignored_tuple_struct: SomeIgnoredTupleStruct,
        ignored_struct_variant: SomeIgnoredEnum,
        ignored_tuple_variant: SomeIgnoredEnum,
        custom_deserialize: CustomDeserialize,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeStruct {
        foo: i64,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeTupleStruct(String);

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeUnitStruct;

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeIgnoredStruct {
        #[reflect(ignore)]
        ignored: i32,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct SomeIgnoredTupleStruct(#[reflect(ignore)] i32);

    #[derive(Reflect, Debug, PartialEq, Deserialize)]
    struct SomeDeserializableStruct {
        foo: i64,
    }

    /// Implements a custom deserialize using `#[reflect(Deserialize)]`.
    ///
    /// For testing purposes, this is just the auto-generated one from deriving.
    #[derive(Reflect, Debug, PartialEq, Deserialize)]
    #[reflect(Deserialize)]
    struct CustomDeserialize {
        value: usize,
        #[serde(alias = "renamed")]
        inner_struct: SomeDeserializableStruct,
    }

    #[derive(Reflect, Debug, PartialEq)]
    enum SomeEnum {
        Unit,
        NewType(usize),
        Tuple(f32, f32),
        Struct { foo: String },
    }

    #[derive(Reflect, Debug, PartialEq)]
    enum SomeIgnoredEnum {
        Tuple(#[reflect(ignore)] f32, #[reflect(ignore)] f32),
        Struct {
            #[reflect(ignore)]
            foo: String,
        },
    }

    fn get_registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();
        registry.register::<SomeStruct>();
        registry.register::<SomeTupleStruct>();
        registry.register::<SomeUnitStruct>();
        registry.register::<SomeIgnoredStruct>();
        registry.register::<SomeIgnoredTupleStruct>();
        registry.register::<CustomDeserialize>();
        registry.register::<SomeDeserializableStruct>();
        registry.register::<SomeEnum>();
        registry.register::<SomeIgnoredEnum>();
        registry.register::<i8>();
        registry.register::<String>();
        registry.register::<i64>();
        registry.register::<f32>();
        registry.register::<usize>();
        registry.register::<i32>();
        registry.register::<u8>();
        registry.register::<(f32, usize)>();
        registry.register::<[i32; 5]>();
        registry.register::<Vec<i32>>();
        registry.register::<HashMap<u8, usize>>();
        registry.register::<HashSet<u8>>();
        registry.register::<Option<SomeStruct>>();
        registry.register::<Option<String>>();
        registry.register_type_data::<Option<String>, ReflectDeserialize>();
        registry
    }

    fn get_my_struct() -> MyStruct {
        let mut map = HashMap::new();
        map.insert(64, 32);

        let mut set = HashSet::new();
        set.insert(64);

        MyStruct {
            primitive_value: 123,
            option_value: Some(String::from("Hello world!")),
            option_value_complex: Some(SomeStruct { foo: 123 }),
            tuple_value: (PI, 1337),
            list_value: vec![-2, -1, 0, 1, 2],
            array_value: [-2, -1, 0, 1, 2],
            map_value: map,
            set_value: set,
            struct_value: SomeStruct { foo: 999999999 },
            tuple_struct_value: SomeTupleStruct(String::from("Tuple Struct")),
            unit_struct: SomeUnitStruct,
            unit_enum: SomeEnum::Unit,
            newtype_enum: SomeEnum::NewType(123),
            tuple_enum: SomeEnum::Tuple(1.23, 3.21),
            struct_enum: SomeEnum::Struct {
                foo: String::from("Struct variant value"),
            },
            ignored_struct: SomeIgnoredStruct { ignored: 0 },
            ignored_tuple_struct: SomeIgnoredTupleStruct(0),
            ignored_struct_variant: SomeIgnoredEnum::Struct {
                foo: String::default(),
            },
            ignored_tuple_variant: SomeIgnoredEnum::Tuple(0.0, 0.0),
            custom_deserialize: CustomDeserialize {
                value: 100,
                inner_struct: SomeDeserializableStruct { foo: 101 },
            },
        }
    }

    #[test]
    fn should_deserialize() {
        let expected = get_my_struct();
        let registry = get_registry();

        let input = r#"{
            "bevy_reflect::serde::de::tests::MyStruct": (
                primitive_value: 123,
                option_value: Some("Hello world!"),
                option_value_complex: Some((
                    foo: 123,
                )),
                tuple_value: (3.1415927, 1337),
                list_value: [
                    -2,
                    -1,
                    0,
                    1,
                    2,
                ],
                array_value: (-2, -1, 0, 1, 2),
                map_value: {
                    64: 32,
                },
                set_value: [
                    64,
                ],
                struct_value: (
                    foo: 999999999,
                ),
                tuple_struct_value: ("Tuple Struct"),
                unit_struct: (),
                unit_enum: Unit,
                newtype_enum: NewType(123),
                tuple_enum: Tuple(1.23, 3.21),
                struct_enum: Struct(
                    foo: "Struct variant value",
                ),
                ignored_struct: (),
                ignored_tuple_struct: (),
                ignored_struct_variant: Struct(),
                ignored_tuple_variant: Tuple(),
                custom_deserialize: (
                    value: 100,
                    renamed: (
                        foo: 101,
                    ),
                ),
            ),
        }"#;

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output = <MyStruct as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize_value() {
        let input = r#"{
            "f32": 1.23,
        }"#;

        let registry = get_registry();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();
        let output = dynamic_output
            .try_take::<f32>()
            .expect("underlying type should be f32");
        assert_eq!(1.23, output);
    }

    #[test]
    fn should_deserialized_typed() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo {
            bar: i32,
        }

        let expected = Foo { bar: 123 };

        let input = r#"(
            bar: 123
        )"#;

        let mut registry = get_registry();
        registry.register::<Foo>();
        let registration = registry.get(TypeId::of::<Foo>()).unwrap();
        let reflect_deserializer = TypedReflectDeserializer::new(registration, &registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output =
            <Foo as FromReflect>::from_reflect(dynamic_output.as_ref().as_partial_reflect())
                .unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize_option() {
        #[derive(Reflect, Debug, PartialEq)]
        struct OptionTest {
            none: Option<()>,
            simple: Option<String>,
            complex: Option<SomeStruct>,
        }

        let expected = OptionTest {
            none: None,
            simple: Some(String::from("Hello world!")),
            complex: Some(SomeStruct { foo: 123 }),
        };

        let mut registry = get_registry();
        registry.register::<OptionTest>();
        registry.register::<Option<()>>();

        // === Normal === //
        let input = r#"{
            "bevy_reflect::serde::de::tests::OptionTest": (
                none: None,
                simple: Some("Hello world!"),
                complex: Some((
                    foo: 123,
                )),
            ),
        }"#;

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output = <OptionTest as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output, "failed to deserialize Options");

        // === Implicit Some === //
        let input = r#"
        #![enable(implicit_some)]
        {
            "bevy_reflect::serde::de::tests::OptionTest": (
                none: None,
                simple: "Hello world!",
                complex: (
                    foo: 123,
                ),
            ),
        }"#;

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut ron_deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let dynamic_output = reflect_deserializer
            .deserialize(&mut ron_deserializer)
            .unwrap();

        let output = <OptionTest as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(
            expected, output,
            "failed to deserialize Options with implicit Some"
        );
    }

    #[test]
    fn enum_should_deserialize() {
        #[derive(Reflect)]
        enum MyEnum {
            Unit,
            NewType(usize),
            Tuple(f32, f32),
            Struct { value: String },
        }

        let mut registry = get_registry();
        registry.register::<MyEnum>();

        // === Unit Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": Unit,
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Unit);
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === NewType Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": NewType(123),
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::NewType(123));
        assert!(expected.reflect_partial_eq(output.as_ref()).unwrap());

        // === Tuple Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": Tuple(1.23, 3.21),
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Tuple(1.23, 3.21));
        assert!(expected
            .reflect_partial_eq(output.as_partial_reflect())
            .unwrap());

        // === Struct Variant === //
        let input = r#"{
    "bevy_reflect::serde::de::tests::MyEnum": Struct(
        value: "I <3 Enums",
    ),
}"#;
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let output = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let expected = DynamicEnum::from(MyEnum::Struct {
            value: String::from("I <3 Enums"),
        });
        assert!(expected
            .reflect_partial_eq(output.as_partial_reflect())
            .unwrap());
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/12462
    #[test]
    fn should_reserialize() {
        let registry = get_registry();
        let input1 = get_my_struct();

        let serializer1 = ReflectSerializer::new(&input1, &registry);
        let serialized1 = ron::ser::to_string(&serializer1).unwrap();

        let mut deserializer = ron::de::Deserializer::from_str(&serialized1).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let input2 = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let serializer2 = ReflectSerializer::new(input2.as_partial_reflect(), &registry);
        let serialized2 = ron::ser::to_string(&serializer2).unwrap();

        assert_eq!(serialized1, serialized2);
    }

    #[test]
    fn should_deserialize_non_self_describing_binary() {
        let expected = get_my_struct();
        let registry = get_registry();

        let input = vec![
            1, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 98, 101, 118, 121, 95, 114, 101, 102,
            108, 101, 99, 116, 58, 58, 115, 101, 114, 100, 101, 58, 58, 100, 101, 58, 58, 116, 101,
            115, 116, 115, 58, 58, 77, 121, 83, 116, 114, 117, 99, 116, 123, 1, 12, 0, 0, 0, 0, 0,
            0, 0, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33, 1, 123, 0, 0, 0, 0, 0,
            0, 0, 219, 15, 73, 64, 57, 5, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 254, 255, 255,
            255, 255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 254, 255, 255, 255, 255,
            255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 32, 0,
            0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 255, 201, 154, 59, 0, 0, 0, 0, 12, 0, 0,
            0, 0, 0, 0, 0, 84, 117, 112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 0, 0, 0, 0, 1,
            0, 0, 0, 123, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 164, 112, 157, 63, 164, 112, 77, 64, 3,
            0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105,
            97, 110, 116, 32, 118, 97, 108, 117, 101, 1, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0,
            0, 0, 101, 0, 0, 0, 0, 0, 0, 0,
        ];

        let deserializer = ReflectDeserializer::new(&registry);

        let dynamic_output = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize_seed(deserializer, &input)
            .unwrap();

        let output = <MyStruct as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize_self_describing_binary() {
        let expected = get_my_struct();
        let registry = get_registry();

        let input = vec![
            129, 217, 40, 98, 101, 118, 121, 95, 114, 101, 102, 108, 101, 99, 116, 58, 58, 115,
            101, 114, 100, 101, 58, 58, 100, 101, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77, 121,
            83, 116, 114, 117, 99, 116, 220, 0, 20, 123, 172, 72, 101, 108, 108, 111, 32, 119, 111,
            114, 108, 100, 33, 145, 123, 146, 202, 64, 73, 15, 219, 205, 5, 57, 149, 254, 255, 0,
            1, 2, 149, 254, 255, 0, 1, 2, 129, 64, 32, 145, 64, 145, 206, 59, 154, 201, 255, 145,
            172, 84, 117, 112, 108, 101, 32, 83, 116, 114, 117, 99, 116, 144, 164, 85, 110, 105,
            116, 129, 167, 78, 101, 119, 84, 121, 112, 101, 123, 129, 165, 84, 117, 112, 108, 101,
            146, 202, 63, 157, 112, 164, 202, 64, 77, 112, 164, 129, 166, 83, 116, 114, 117, 99,
            116, 145, 180, 83, 116, 114, 117, 99, 116, 32, 118, 97, 114, 105, 97, 110, 116, 32,
            118, 97, 108, 117, 101, 144, 144, 129, 166, 83, 116, 114, 117, 99, 116, 144, 129, 165,
            84, 117, 112, 108, 101, 144, 146, 100, 145, 101,
        ];

        let mut reader = std::io::BufReader::new(input.as_slice());

        let deserializer = ReflectDeserializer::new(&registry);
        let dynamic_output = deserializer
            .deserialize(&mut rmp_serde::Deserializer::new(&mut reader))
            .unwrap();

        let output = <MyStruct as FromReflect>::from_reflect(dynamic_output.as_ref()).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_return_error_if_missing_type_data() {
        let mut registry = TypeRegistry::new();
        registry.register::<RangeInclusive<f32>>();

        let input = r#"{"core::ops::RangeInclusive<f32>":(start:0.0,end:1.0)}"#;
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let error = reflect_deserializer
            .deserialize(&mut deserializer)
            .unwrap_err();
        assert_eq!(error, ron::Error::Message("Type `core::ops::RangeInclusive<f32>` did not register the `ReflectDeserialize` type data. For certain types, this may need to be registered manually using `register_type_data`".to_string()));
    }
}
