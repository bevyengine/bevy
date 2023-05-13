mod de;
mod ser;
mod type_data;

pub use de::*;
pub use ser::*;
pub use type_data::*;

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_reflect,
        serde::{
            DeserializeReflect, ReflectDeserializeReflect, ReflectDeserializer, ReflectSerializer,
            TypedReflectDeserializer,
        },
        type_registry::TypeRegistry,
        DynamicStruct, DynamicTupleStruct, FromReflect, PartialReflect, Reflect,
        ReflectDeserialize, Struct,
    };
    use core::fmt::Formatter;
    use serde::de::{DeserializeSeed, SeqAccess, Visitor};
    use serde::{Deserialize, Deserializer};

    #[test]
    fn test_serialization_struct() {
        #[derive(Debug, Reflect, PartialEq)]
        #[reflect(PartialEq)]
        struct TestStruct {
            a: i32,
            #[reflect(ignore)]
            b: i32,
            #[reflect(skip_serializing)]
            c: i32,
            #[reflect(skip_serializing)]
            #[reflect(default = "custom_default")]
            d: i32,
            e: i32,
        }

        fn custom_default() -> i32 {
            -1
        }

        let mut registry = TypeRegistry::default();
        registry.register::<TestStruct>();

        let test_struct = TestStruct {
            a: 3,
            b: 4,
            c: 5,
            d: 6,
            e: 7,
        };

        let serializer = ReflectSerializer::new(&test_struct, &registry);
        let serialized =
            ron::ser::to_string_pretty(&serializer, ron::ser::PrettyConfig::default()).unwrap();

        let mut deserializer = ron::de::Deserializer::from_str(&serialized).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let deserialized = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let mut expected = DynamicStruct::default();
        expected.insert("a", 3);
        // Ignored: expected.insert("b", 0);
        expected.insert("c", 0);
        expected.insert("d", -1);
        expected.insert("e", 7);

        assert!(
            expected
                .reflect_partial_eq(deserialized.as_partial_reflect())
                .unwrap(),
            "Deserialization failed: expected {expected:?} found {deserialized:?}"
        );

        let expected = TestStruct {
            a: 3,
            b: 0,
            c: 0,
            d: -1,
            e: 7,
        };
        let received =
            <TestStruct as FromReflect>::from_reflect(deserialized.as_partial_reflect()).unwrap();

        assert_eq!(
            expected, received,
            "FromReflect failed: expected {expected:?} found {received:?}"
        );
    }

    #[test]
    fn test_serialization_tuple_struct() {
        #[derive(Debug, Reflect, PartialEq)]
        #[reflect(PartialEq)]
        struct TestStruct(
            i32,
            #[reflect(ignore)] i32,
            #[reflect(skip_serializing)] i32,
            #[reflect(skip_serializing)]
            #[reflect(default = "custom_default")]
            i32,
            i32,
        );

        fn custom_default() -> i32 {
            -1
        }

        let mut registry = TypeRegistry::default();
        registry.register::<TestStruct>();

        let test_struct = TestStruct(3, 4, 5, 6, 7);

        let serializer = ReflectSerializer::new(&test_struct, &registry);
        let serialized =
            ron::ser::to_string_pretty(&serializer, ron::ser::PrettyConfig::default()).unwrap();

        let mut deserializer = ron::de::Deserializer::from_str(&serialized).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let deserialized = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        let mut expected = DynamicTupleStruct::default();
        expected.insert(3);
        // Ignored: expected.insert(0);
        expected.insert(0);
        expected.insert(-1);
        expected.insert(7);

        assert!(
            expected
                .reflect_partial_eq(deserialized.as_partial_reflect())
                .unwrap(),
            "Deserialization failed: expected {expected:?} found {deserialized:?}"
        );

        let expected = TestStruct(3, 0, 0, -1, 7);
        let received =
            <TestStruct as FromReflect>::from_reflect(deserialized.as_partial_reflect()).unwrap();

        assert_eq!(
            expected, received,
            "FromReflect failed: expected {expected:?} found {received:?}"
        );
    }

    #[test]
    #[should_panic(
        expected = "cannot serialize dynamic value without represented type: `bevy_reflect::DynamicStruct`"
    )]
    fn should_not_serialize_unproxied_dynamic() {
        let registry = TypeRegistry::default();

        let mut value = DynamicStruct::default();
        value.insert("foo", 123_u32);

        let serializer = ReflectSerializer::new(&value, &registry);
        ron::ser::to_string(&serializer).unwrap();
    }

    #[test]
    fn should_roundtrip_proxied_dynamic() {
        #[derive(Reflect)]
        struct TestStruct {
            a: i32,
            b: i32,
        }

        let mut registry = TypeRegistry::default();
        registry.register::<TestStruct>();

        let value: DynamicStruct = TestStruct { a: 123, b: 456 }.clone_dynamic();

        let serializer = ReflectSerializer::new(&value, &registry);

        let expected = r#"{"bevy_reflect::serde::tests::TestStruct":(a:123,b:456)}"#;
        let result = ron::ser::to_string(&serializer).unwrap();
        assert_eq!(expected, result);

        let mut deserializer = ron::de::Deserializer::from_str(&result).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);

        let expected = value.clone_value();
        let result = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        assert!(expected
            .reflect_partial_eq(result.as_partial_reflect())
            .unwrap());
    }

    #[test]
    fn should_deserialize_using_deserialize_reflect() {
        #[derive(Reflect, PartialEq, Debug, Deserialize)]
        #[reflect(Deserialize)]
        enum AnimalType {
            Dog,
            Cat,
        }

        #[derive(Reflect)]
        struct Dog {
            name: DogName,
        }

        #[derive(Reflect)]
        enum DogName {
            Spot,
            Fido,
            Rex,
        }

        #[derive(Reflect)]
        struct Cat {
            name: CatName,
        }

        #[derive(Reflect)]
        enum CatName {
            Fluffy,
            Snowball,
            Luna,
        }

        /// Pet is made up of two fields: the type of animal and the animal itself.
        ///
        /// This allows us to store a type-erased version of our pet,
        /// rather than having to define one like this:
        ///
        /// ```
        /// # use bevy_reflect::prelude::Reflect;
        /// #[derive(Reflect)]
        /// struct Pet<T: Reflect>(T);
        /// ```
        ///
        /// If we wanted to allow for deserialization of any type,
        /// we could replace `AnimalType` with a `String` containing the type name of the animal.
        #[derive(Reflect)]
        #[reflect(DeserializeReflect)]
        #[reflect(from_reflect = false)]
        struct Pet(AnimalType, DynamicStruct);

        impl<'de> DeserializeReflect<'de> for Pet {
            fn deserialize<D>(deserializer: D, registry: &TypeRegistry) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct PetVisitor<'a> {
                    registry: &'a TypeRegistry,
                }
                impl<'a, 'de> Visitor<'de> for PetVisitor<'a> {
                    type Value = Pet;

                    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
                        write!(formatter, "a pet tuple struct")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: SeqAccess<'de>,
                    {
                        let kind = seq.next_element::<AnimalType>()?.unwrap();
                        match kind {
                            AnimalType::Cat => {
                                let cat = seq
                                    .next_element_seed(TypedReflectDeserializer::of::<Cat>(
                                        self.registry,
                                    ))?
                                    .unwrap()
                                    .reflect_owned()
                                    .into_struct()
                                    .unwrap()
                                    .clone_dynamic();
                                Ok(Pet(kind, cat))
                            }
                            AnimalType::Dog => {
                                let dog = seq
                                    .next_element_seed(TypedReflectDeserializer::of::<Dog>(
                                        self.registry,
                                    ))?
                                    .unwrap()
                                    .reflect_owned()
                                    .into_struct()
                                    .unwrap()
                                    .clone_dynamic();
                                Ok(Pet(kind, dog))
                            }
                        }
                    }
                }

                deserializer.deserialize_tuple_struct("Pet", 1, PetVisitor { registry })
            }
        }

        let mut registry = TypeRegistry::default();
        registry.register::<Pet>();
        registry.register::<AnimalType>();
        registry.register::<Dog>();
        registry.register::<DogName>();
        registry.register::<Cat>();
        registry.register::<CatName>();

        let pet = Pet(
            AnimalType::Cat,
            Cat {
                name: CatName::Fluffy,
            }
            .clone_dynamic(),
        );

        let serializer = ReflectSerializer::new(&pet, &registry);
        let serialized = ron::ser::to_string(&serializer).unwrap();

        let expected = r#"{"bevy_reflect::serde::tests::Pet":(Cat,(name:Fluffy))}"#;

        assert_eq!(expected, serialized);

        let mut deserializer = ron::de::Deserializer::from_str(&serialized).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
        let deserialized = value.try_take::<Pet>().unwrap();

        assert_eq!(pet.0, deserialized.0);
        assert!(pet
            .1
            .reflect_partial_eq(&deserialized.1)
            .unwrap_or_default());
    }
}
