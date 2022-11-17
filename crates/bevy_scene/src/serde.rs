use crate::{DynamicEntity, DynamicScene};
use anyhow::Result;
use bevy_reflect::serde::{TypedReflectDeserializer, TypedReflectSerializer};
use bevy_reflect::{serde::UntypedReflectDeserializer, Reflect, TypeRegistry, TypeRegistryArc};
use bevy_utils::HashSet;
use serde::ser::SerializeMap;
use serde::{
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::Formatter;

pub const SCENE_STRUCT: &str = "Scene";
pub const SCENE_ENTITIES: &str = "entities";

pub const ENTITY_STRUCT: &str = "Entity";
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

pub struct SceneSerializer<'a> {
    pub scene: &'a DynamicScene,
    pub registry: &'a TypeRegistryArc,
}

impl<'a> SceneSerializer<'a> {
    pub fn new(scene: &'a DynamicScene, registry: &'a TypeRegistryArc) -> Self {
        SceneSerializer { scene, registry }
    }
}

impl<'a> Serialize for SceneSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(SCENE_STRUCT, 1)?;
        state.serialize_field(
            SCENE_ENTITIES,
            &EntitiesSerializer {
                entities: &self.scene.entities,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct EntitiesSerializer<'a> {
    pub entities: &'a [DynamicEntity],
    pub registry: &'a TypeRegistryArc,
}

impl<'a> Serialize for EntitiesSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.entities.len()))?;
        for entity in self.entities {
            state.serialize_entry(
                &entity.entity,
                &EntitySerializer {
                    entity,
                    registry: self.registry,
                },
            )?;
        }
        state.end()
    }
}

pub struct EntitySerializer<'a> {
    pub entity: &'a DynamicEntity,
    pub registry: &'a TypeRegistryArc,
}

impl<'a> Serialize for EntitySerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(ENTITY_STRUCT, 1)?;
        state.serialize_field(
            ENTITY_FIELD_COMPONENTS,
            &ComponentsSerializer {
                components: &self.entity.components,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct ComponentsSerializer<'a> {
    pub components: &'a [Box<dyn Reflect>],
    pub registry: &'a TypeRegistryArc,
}

impl<'a> Serialize for ComponentsSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.components.len()))?;
        for component in self.components {
            state.serialize_entry(
                component.type_name(),
                &TypedReflectSerializer::new(&**component, &self.registry.read()),
            )?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum SceneField {
    Entities,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
    Components,
}

pub struct SceneDeserializer<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneDeserializer<'a> {
    type Value = DynamicScene;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            SCENE_STRUCT,
            &[SCENE_ENTITIES],
            SceneVisitor {
                type_registry: self.type_registry,
            },
        )
    }
}

struct SceneVisitor<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SceneVisitor<'a> {
    type Value = DynamicScene;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("scene struct")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entities = None;
        while let Some(key) = map.next_key()? {
            match key {
                SceneField::Entities => {
                    if entities.is_some() {
                        return Err(Error::duplicate_field(SCENE_ENTITIES));
                    }
                    entities = Some(map.next_value_seed(SceneEntitiesDeserializer {
                        type_registry: self.type_registry,
                    })?);
                }
            }
        }

        let entities = entities.ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene { entities })
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let entities = seq
            .next_element_seed(SceneEntitiesDeserializer {
                type_registry: self.type_registry,
            })?
            .ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene { entities })
    }
}

pub struct SceneEntitiesDeserializer<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneEntitiesDeserializer<'a> {
    type Value = Vec<DynamicEntity>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SceneEntitiesVisitor {
            type_registry: self.type_registry,
        })
    }
}

struct SceneEntitiesVisitor<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SceneEntitiesVisitor<'a> {
    type Value = Vec<DynamicEntity>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("map of entities")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entities = Vec::new();
        while let Some(id) = map.next_key::<u32>()? {
            let entity = map.next_value_seed(SceneEntityDeserializer {
                id,
                type_registry: self.type_registry,
            })?;
            entities.push(entity);
        }

        Ok(entities)
    }
}

pub struct SceneEntityDeserializer<'a> {
    pub id: u32,
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneEntityDeserializer<'a> {
    type Value = DynamicEntity;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            ENTITY_STRUCT,
            &[ENTITY_FIELD_COMPONENTS],
            SceneEntityVisitor {
                id: self.id,
                registry: self.type_registry,
            },
        )
    }
}

struct SceneEntityVisitor<'a> {
    pub id: u32,
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SceneEntityVisitor<'a> {
    type Value = DynamicEntity;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let components = seq
            .next_element_seed(ComponentDeserializer {
                registry: self.registry,
            })?
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;

        Ok(DynamicEntity {
            entity: self.id,
            components,
        })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut components = None;
        while let Some(key) = map.next_key()? {
            match key {
                EntityField::Components => {
                    if components.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_COMPONENTS));
                    }

                    components = Some(map.next_value_seed(ComponentDeserializer {
                        registry: self.registry,
                    })?);
                }
            }
        }

        let components = components
            .take()
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
        Ok(DynamicEntity {
            entity: self.id,
            components,
        })
    }
}

pub struct ComponentDeserializer<'a> {
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentDeserializer<'a> {
    type Value = Vec<Box<dyn Reflect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ComponentVisitor {
            registry: self.registry,
        })
    }
}

struct ComponentVisitor<'a> {
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ComponentVisitor<'a> {
    type Value = Vec<Box<dyn Reflect>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("map of components")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut added = HashSet::new();
        let mut components = Vec::new();
        while let Some(key) = map.next_key::<&str>()? {
            if !added.insert(key) {
                return Err(Error::custom(format!("duplicate component: `{key}`")));
            }

            let registration = self
                .registry
                .get_with_name(key)
                .ok_or_else(|| Error::custom(format!("no registration found for `{key}`")))?;
            components.push(
                map.next_value_seed(TypedReflectDeserializer::new(registration, self.registry))?,
            );
        }

        Ok(components)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut dynamic_properties = Vec::new();
        while let Some(entity) =
            seq.next_element_seed(UntypedReflectDeserializer::new(self.registry))?
        {
            dynamic_properties.push(entity);
        }

        Ok(dynamic_properties)
    }
}

#[cfg(test)]
mod tests {
    use crate::serde::{SceneDeserializer, SceneSerializer};
    use crate::{DynamicScene, DynamicSceneBuilder};
    use bevy_app::AppTypeRegistry;
    use bevy_ecs::entity::EntityMap;
    use bevy_ecs::prelude::{Component, ReflectComponent, World};
    use bevy_reflect::{FromReflect, Reflect, ReflectSerialize};
    use bincode::Options;
    use serde::de::DeserializeSeed;

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct Foo(i32);
    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct Bar(i32);
    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct Baz(i32);

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct MyComponent {
        foo: [usize; 3],
        bar: (f32, f32),
        baz: MyEnum,
    }

    #[derive(Reflect, FromReflect, Default)]
    enum MyEnum {
        #[default]
        Unit,
        Tuple(String),
        Struct {
            value: u32,
        },
    }

    fn create_world() -> World {
        let mut world = World::new();
        let registry = AppTypeRegistry::default();
        {
            let mut registry = registry.write();
            registry.register::<Foo>();
            registry.register::<Bar>();
            registry.register::<Baz>();
            registry.register::<MyComponent>();
            registry.register::<MyEnum>();
            registry.register::<String>();
            registry.register_type_data::<String, ReflectSerialize>();
            registry.register::<[usize; 3]>();
            registry.register::<(f32, f32)>();
        }
        world.insert_resource(registry);
        world
    }

    #[test]
    fn should_serialize() {
        let mut world = create_world();

        let a = world.spawn(Foo(123)).id();
        let b = world.spawn((Foo(123), Bar(345))).id();
        let c = world.spawn((Foo(123), Bar(345), Baz(789))).id();

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_entities([a, b, c].into_iter());
        let scene = builder.build();

        let expected = r#"(
  entities: {
    0: (
      components: {
        "bevy_scene::serde::tests::Foo": (123),
      },
    ),
    1: (
      components: {
        "bevy_scene::serde::tests::Foo": (123),
        "bevy_scene::serde::tests::Bar": (345),
      },
    ),
    2: (
      components: {
        "bevy_scene::serde::tests::Foo": (123),
        "bevy_scene::serde::tests::Bar": (345),
        "bevy_scene::serde::tests::Baz": (789),
      },
    ),
  },
)"#;
        let output = scene
            .serialize_ron(&world.resource::<AppTypeRegistry>().0)
            .unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_deserialize() {
        let world = create_world();

        let input = r#"(
  entities: {
    0: (
      components: {
        "bevy_scene::serde::tests::Foo": (123),
      },
    ),
    1: (
      components: {
        "bevy_scene::serde::tests::Foo": (123),
        "bevy_scene::serde::tests::Bar": (345),
      },
    ),
    2: (
      components: {
        "bevy_scene::serde::tests::Foo": (123),
        "bevy_scene::serde::tests::Bar": (345),
        "bevy_scene::serde::tests::Baz": (789),
      },
    ),
  },
)"#;
        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let scene_deserializer = SceneDeserializer {
            type_registry: &world.resource::<AppTypeRegistry>().read(),
        };
        let scene = scene_deserializer.deserialize(&mut deserializer).unwrap();

        assert_eq!(
            3,
            scene.entities.len(),
            "expected `entities` to contain 3 entities"
        );

        let mut map = EntityMap::default();
        let mut dst_world = create_world();
        scene.write_to_world(&mut dst_world, &mut map).unwrap();

        assert_eq!(3, dst_world.query::<&Foo>().iter(&dst_world).count());
        assert_eq!(2, dst_world.query::<&Bar>().iter(&dst_world).count());
        assert_eq!(1, dst_world.query::<&Baz>().iter(&dst_world).count());
    }

    #[test]
    fn should_roundtrip_postcard() {
        let mut world = create_world();

        world.spawn(MyComponent {
            foo: [1, 2, 3],
            bar: (1.3, 3.7),
            baz: MyEnum::Tuple("Hello World!".to_string()),
        });

        let registry = world.resource::<AppTypeRegistry>();

        let scene = DynamicScene::from_world(&world, registry);

        let scene_serializer = SceneSerializer::new(&scene, &registry.0);
        let serialized_scene = postcard::to_allocvec(&scene_serializer).unwrap();

        assert_eq!(
            vec![
                1, 0, 1, 37, 98, 101, 118, 121, 95, 115, 99, 101, 110, 101, 58, 58, 115, 101, 114,
                100, 101, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77, 121, 67, 111, 109, 112, 111,
                110, 101, 110, 116, 1, 2, 3, 102, 102, 166, 63, 205, 204, 108, 64, 1, 12, 72, 101,
                108, 108, 111, 32, 87, 111, 114, 108, 100, 33
            ],
            serialized_scene
        );

        let scene_deserializer = SceneDeserializer {
            type_registry: &registry.0.read(),
        };
        let deserialized_scene = scene_deserializer
            .deserialize(&mut postcard::Deserializer::from_bytes(&serialized_scene))
            .unwrap();

        assert_eq!(1, deserialized_scene.entities.len());
        assert_scene_eq(&scene, &deserialized_scene);
    }

    #[test]
    fn should_roundtrip_bincode() {
        let mut world = create_world();

        world.spawn(MyComponent {
            foo: [1, 2, 3],
            bar: (1.3, 3.7),
            baz: MyEnum::Tuple("Hello World!".to_string()),
        });

        let registry = world.resource::<AppTypeRegistry>();

        let scene = DynamicScene::from_world(&world, registry);

        let scene_serializer = SceneSerializer::new(&scene, &registry.0);
        let serialized_scene = bincode::serialize(&scene_serializer).unwrap();

        assert_eq!(
            vec![
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 37, 0, 0, 0, 0, 0, 0,
                0, 98, 101, 118, 121, 95, 115, 99, 101, 110, 101, 58, 58, 115, 101, 114, 100, 101,
                58, 58, 116, 101, 115, 116, 115, 58, 58, 77, 121, 67, 111, 109, 112, 111, 110, 101,
                110, 116, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
                102, 102, 166, 63, 205, 204, 108, 64, 1, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 72, 101,
                108, 108, 111, 32, 87, 111, 114, 108, 100, 33
            ],
            serialized_scene
        );

        let scene_deserializer = SceneDeserializer {
            type_registry: &registry.0.read(),
        };

        let deserialized_scene = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize_seed(scene_deserializer, &serialized_scene)
            .unwrap();

        assert_eq!(1, deserialized_scene.entities.len());
        assert_scene_eq(&scene, &deserialized_scene);
    }

    /// A crude equality checker for [`DynamicScene`], used solely for testing purposes.
    fn assert_scene_eq(expected: &DynamicScene, received: &DynamicScene) {
        assert_eq!(
            expected.entities.len(),
            received.entities.len(),
            "entity count did not match",
        );

        for expected in &expected.entities {
            let received = received
                .entities
                .iter()
                .find(|dynamic_entity| dynamic_entity.entity == expected.entity)
                .unwrap_or_else(|| panic!("missing entity (expected: `{}`)", expected.entity));

            assert_eq!(expected.entity, received.entity, "entities did not match",);

            for expected in &expected.components {
                let received = received
                    .components
                    .iter()
                    .find(|component| component.type_name() == expected.type_name())
                    .unwrap_or_else(|| {
                        panic!("missing component (expected: `{}`)", expected.type_name())
                    });

                assert!(
                    expected
                        .reflect_partial_eq(received.as_ref())
                        .unwrap_or_default(),
                    "components did not match: (expected: `{:?}`, received: `{:?}`)",
                    expected,
                    received
                );
            }
        }
    }

    /// These tests just verify that that the [`assert_scene_eq`] function is working properly for our tests.
    mod assert_scene_eq_tests {
        use super::*;

        #[test]
        #[should_panic(expected = "entity count did not match")]
        fn should_panic_when_entity_count_not_eq() {
            let mut world = create_world();
            let registry = world.resource::<AppTypeRegistry>();
            let scene_a = DynamicScene::from_world(&world, registry);

            world.spawn(MyComponent {
                foo: [1, 2, 3],
                bar: (1.3, 3.7),
                baz: MyEnum::Unit,
            });

            let registry = world.resource::<AppTypeRegistry>();
            let scene_b = DynamicScene::from_world(&world, registry);

            assert_scene_eq(&scene_a, &scene_b);
        }

        #[test]
        #[should_panic(expected = "components did not match")]
        fn should_panic_when_components_not_eq() {
            let mut world = create_world();

            let entity = world
                .spawn(MyComponent {
                    foo: [1, 2, 3],
                    bar: (1.3, 3.7),
                    baz: MyEnum::Unit,
                })
                .id();

            let registry = world.resource::<AppTypeRegistry>();
            let scene_a = DynamicScene::from_world(&world, registry);

            world.entity_mut(entity).insert(MyComponent {
                foo: [3, 2, 1],
                bar: (1.3, 3.7),
                baz: MyEnum::Unit,
            });

            let registry = world.resource::<AppTypeRegistry>();
            let scene_b = DynamicScene::from_world(&world, registry);

            assert_scene_eq(&scene_a, &scene_b);
        }

        #[test]
        #[should_panic(expected = "missing component")]
        fn should_panic_when_missing_component() {
            let mut world = create_world();

            let entity = world
                .spawn(MyComponent {
                    foo: [1, 2, 3],
                    bar: (1.3, 3.7),
                    baz: MyEnum::Unit,
                })
                .id();

            let registry = world.resource::<AppTypeRegistry>();
            let scene_a = DynamicScene::from_world(&world, registry);

            world.entity_mut(entity).remove::<MyComponent>();

            let registry = world.resource::<AppTypeRegistry>();
            let scene_b = DynamicScene::from_world(&world, registry);

            assert_scene_eq(&scene_a, &scene_b);
        }
    }
}
