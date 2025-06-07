//! Serde integration for reflected types.

mod de;
mod ser;
mod type_data;

pub use de::*;
pub use ser::*;
pub use type_data::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        type_registry::TypeRegistry, DynamicStruct, DynamicTupleStruct, FromReflect,
        PartialReflect, Reflect, Struct,
    };
    use serde::de::DeserializeSeed;

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

        let value: DynamicStruct = TestStruct { a: 123, b: 456 }.to_dynamic_struct();

        let serializer = ReflectSerializer::new(&value, &registry);

        let expected = r#"{"bevy_reflect::serde::tests::TestStruct":(a:123,b:456)}"#;
        let result = ron::ser::to_string(&serializer).unwrap();
        assert_eq!(expected, result);

        let mut deserializer = ron::de::Deserializer::from_str(&result).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);

        let expected = value.to_dynamic();
        let result = reflect_deserializer.deserialize(&mut deserializer).unwrap();

        assert!(expected
            .reflect_partial_eq(result.as_partial_reflect())
            .unwrap());
    }

    mod type_data {
        use super::*;
        use crate::from_reflect::FromReflect;
        use crate::serde::{DeserializeWithRegistry, ReflectDeserializeWithRegistry};
        use crate::serde::{ReflectSerializeWithRegistry, SerializeWithRegistry};
        use crate::{ReflectFromReflect, TypePath};
        use alloc::{format, string::String, vec, vec::Vec};
        use bevy_platform::sync::Arc;
        use bevy_reflect_derive::reflect_trait;
        use core::any::TypeId;
        use core::fmt::{Debug, Formatter};
        use serde::de::{SeqAccess, Visitor};
        use serde::ser::SerializeSeq;
        use serde::{Deserializer, Serialize, Serializer};

        #[reflect_trait]
        trait Enemy: Reflect + Debug {
            #[expect(dead_code, reason = "this method is purely for testing purposes")]
            fn hp(&self) -> u8;
        }

        // This is needed to support Arc<dyn Enemy>
        impl TypePath for dyn Enemy {
            fn type_path() -> &'static str {
                "dyn bevy_reflect::serde::tests::type_data::Enemy"
            }

            fn short_type_path() -> &'static str {
                "dyn Enemy"
            }
        }

        #[derive(Reflect, Debug)]
        #[reflect(Enemy)]
        struct Skeleton(u8);

        impl Enemy for Skeleton {
            fn hp(&self) -> u8 {
                self.0
            }
        }

        #[derive(Reflect, Debug)]
        #[reflect(Enemy)]
        struct Zombie {
            health: u8,
            walk_speed: f32,
        }

        impl Enemy for Zombie {
            fn hp(&self) -> u8 {
                self.health
            }
        }

        #[derive(Reflect, Debug)]
        struct Level {
            name: String,
            enemies: EnemyList,
        }

        #[derive(Reflect, Debug)]
        #[reflect(SerializeWithRegistry, DeserializeWithRegistry)]
        // Note that we have to use `Arc` instead of `Box` here due to the
        // former being the only one between the two to implement `Reflect`.
        struct EnemyList(Vec<Arc<dyn Enemy>>);

        impl SerializeWithRegistry for EnemyList {
            fn serialize<S>(
                &self,
                serializer: S,
                registry: &TypeRegistry,
            ) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let mut state = serializer.serialize_seq(Some(self.0.len()))?;
                for enemy in &self.0 {
                    state.serialize_element(&ReflectSerializer::new(
                        (**enemy).as_partial_reflect(),
                        registry,
                    ))?;
                }
                state.end()
            }
        }

        impl<'de> DeserializeWithRegistry<'de> for EnemyList {
            fn deserialize<D>(deserializer: D, registry: &TypeRegistry) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct EnemyListVisitor<'a> {
                    registry: &'a TypeRegistry,
                }

                impl<'a, 'de> Visitor<'de> for EnemyListVisitor<'a> {
                    type Value = Vec<Arc<dyn Enemy>>;

                    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
                        write!(formatter, "a list of enemies")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: SeqAccess<'de>,
                    {
                        let mut enemies = Vec::new();
                        while let Some(enemy) =
                            seq.next_element_seed(ReflectDeserializer::new(self.registry))?
                        {
                            let registration = self
                                .registry
                                .get_with_type_path(
                                    enemy.get_represented_type_info().unwrap().type_path(),
                                )
                                .unwrap();

                            // 1. Convert any possible dynamic values to concrete ones
                            let enemy = registration
                                .data::<ReflectFromReflect>()
                                .unwrap()
                                .from_reflect(&*enemy)
                                .unwrap();

                            // 2. Convert the concrete value to a boxed trait object
                            let enemy = registration
                                .data::<ReflectEnemy>()
                                .unwrap()
                                .get_boxed(enemy)
                                .unwrap();

                            enemies.push(enemy.into());
                        }

                        Ok(enemies)
                    }
                }

                deserializer
                    .deserialize_seq(EnemyListVisitor { registry })
                    .map(EnemyList)
            }
        }

        fn create_registry() -> TypeRegistry {
            let mut registry = TypeRegistry::default();
            registry.register::<Level>();
            registry.register::<EnemyList>();
            registry.register::<Skeleton>();
            registry.register::<Zombie>();
            registry
        }

        fn create_arc_dyn_enemy<T: Enemy>(enemy: T) -> Arc<dyn Enemy> {
            let arc = Arc::new(enemy);

            #[cfg(not(target_has_atomic = "ptr"))]
            #[expect(
                unsafe_code,
                reason = "unsized coercion is an unstable feature for non-std types"
            )]
            // SAFETY:
            // - Coercion from `T` to `dyn Enemy` is valid as `T: Enemy + 'static`
            // - `Arc::from_raw` receives a valid pointer from a previous call to `Arc::into_raw`
            let arc = unsafe { Arc::from_raw(Arc::into_raw(arc) as *const dyn Enemy) };

            arc
        }

        #[test]
        fn should_serialize_with_serialize_with_registry() {
            let registry = create_registry();

            let level = Level {
                name: String::from("Level 1"),
                enemies: EnemyList(vec![
                    create_arc_dyn_enemy(Skeleton(10)),
                    create_arc_dyn_enemy(Zombie {
                        health: 20,
                        walk_speed: 0.5,
                    }),
                ]),
            };

            let serializer = ReflectSerializer::new(&level, &registry);
            let serialized = ron::ser::to_string(&serializer).unwrap();

            let expected = r#"{"bevy_reflect::serde::tests::type_data::Level":(name:"Level 1",enemies:[{"bevy_reflect::serde::tests::type_data::Skeleton":(10)},{"bevy_reflect::serde::tests::type_data::Zombie":(health:20,walk_speed:0.5)}])}"#;

            assert_eq!(expected, serialized);
        }

        #[test]
        fn should_deserialize_with_deserialize_with_registry() {
            let registry = create_registry();

            let input = r#"{"bevy_reflect::serde::tests::type_data::Level":(name:"Level 1",enemies:[{"bevy_reflect::serde::tests::type_data::Skeleton":(10)},{"bevy_reflect::serde::tests::type_data::Zombie":(health:20,walk_speed:0.5)}])}"#;

            let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
            let reflect_deserializer = ReflectDeserializer::new(&registry);
            let value = reflect_deserializer.deserialize(&mut deserializer).unwrap();

            let output = Level::from_reflect(&*value).unwrap();

            let expected = Level {
                name: String::from("Level 1"),
                enemies: EnemyList(vec![
                    create_arc_dyn_enemy(Skeleton(10)),
                    create_arc_dyn_enemy(Zombie {
                        health: 20,
                        walk_speed: 0.5,
                    }),
                ]),
            };

            // Poor man's comparison since we can't derive PartialEq for Arc<dyn Enemy>
            assert_eq!(format!("{expected:?}"), format!("{output:?}",));

            let unexpected = Level {
                name: String::from("Level 1"),
                enemies: EnemyList(vec![
                    create_arc_dyn_enemy(Skeleton(20)),
                    create_arc_dyn_enemy(Zombie {
                        health: 20,
                        walk_speed: 5.0,
                    }),
                ]),
            };

            // Poor man's comparison since we can't derive PartialEq for Arc<dyn Enemy>
            assert_ne!(format!("{unexpected:?}"), format!("{output:?}"));
        }

        #[test]
        fn should_serialize_single_tuple_struct_as_newtype() {
            #[derive(Reflect, Serialize, PartialEq, Debug)]
            struct TupleStruct(u32);

            #[derive(Reflect, Serialize, PartialEq, Debug)]
            struct TupleStructWithSkip(
                u32,
                #[reflect(skip_serializing)]
                #[serde(skip)]
                u32,
            );

            #[derive(Reflect, Serialize, PartialEq, Debug)]
            enum Enum {
                TupleStruct(usize),
                NestedTupleStruct(TupleStruct),
                NestedTupleStructWithSkip(TupleStructWithSkip),
            }

            let mut registry = TypeRegistry::default();
            registry.register::<TupleStruct>();
            registry.register::<TupleStructWithSkip>();
            registry.register::<Enum>();

            let tuple_struct = TupleStruct(1);
            let tuple_struct_with_skip = TupleStructWithSkip(2, 3);
            let tuple_struct_enum = Enum::TupleStruct(4);
            let nested_tuple_struct = Enum::NestedTupleStruct(TupleStruct(5));
            let nested_tuple_struct_with_skip =
                Enum::NestedTupleStructWithSkip(TupleStructWithSkip(6, 7));

            fn assert_serialize<T: Reflect + FromReflect + Serialize + PartialEq + Debug>(
                value: &T,
                registry: &TypeRegistry,
            ) {
                let serializer = TypedReflectSerializer::new(value, registry);
                let reflect_serialize = serde_json::to_string(&serializer).unwrap();
                let serde_serialize = serde_json::to_string(value).unwrap();
                assert_eq!(reflect_serialize, serde_serialize);

                let registration = registry.get(TypeId::of::<T>()).unwrap();
                let reflect_deserializer = TypedReflectDeserializer::new(registration, registry);

                let mut deserializer = serde_json::Deserializer::from_str(&serde_serialize);
                let reflect_value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
                let _ = T::from_reflect(&*reflect_value).unwrap();
            }

            assert_serialize(&tuple_struct, &registry);
            assert_serialize(&tuple_struct_with_skip, &registry);
            assert_serialize(&tuple_struct_enum, &registry);
            assert_serialize(&nested_tuple_struct, &registry);
            assert_serialize(&nested_tuple_struct_with_skip, &registry);
        }
    }
}
