//! `serde` serialization and deserialization implementation for Bevy scenes.

use crate::{DynamicEntity, DynamicScene};
use bevy_ecs::entity::Entity;
use bevy_reflect::serde::{TypedReflectDeserializer, TypedReflectSerializer};
use bevy_reflect::{
    serde::{TypeRegistrationDeserializer, UntypedReflectDeserializer},
    Reflect, TypeRegistry, TypeRegistryArc,
};
use bevy_utils::HashSet;
use serde::ser::SerializeMap;
use serde::{
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::Formatter;

/// Name of the serialized scene struct type.
pub const SCENE_STRUCT: &str = "Scene";
/// Name of the serialized resources field in a scene struct.
pub const SCENE_RESOURCES: &str = "resources";
/// Name of the serialized entities field in a scene struct.
pub const SCENE_ENTITIES: &str = "entities";

/// Name of the serialized entity struct type.
pub const ENTITY_STRUCT: &str = "Entity";
/// Name of the serialized component field in an entity struct.
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

/// Handles serialization of a scene as a struct containing its entities and resources.
///
/// # Examples
///
/// ```
/// # use bevy_scene::{serde::SceneSerializer, DynamicScene};
/// # use bevy_ecs::{
/// #     prelude::{Component, World},
/// #     reflect::{AppTypeRegistry, ReflectComponent},
/// # };
/// # use bevy_reflect::Reflect;
/// // Define an example component type.
/// #[derive(Component, Reflect, Default)]
/// #[reflect(Component)]
/// struct MyComponent {
///     foo: [usize; 3],
///     bar: (f32, f32),
///     baz: String,
/// }
///
/// // Create our world, provide it with a type registry.
/// // Normally, [`App`] handles providing the type registry.
/// let mut world = World::new();
/// let registry = AppTypeRegistry::default();
/// {
///     let mut registry = registry.write();
///     // Register our component. Primitives and String are registered by default.
///     // Sequence types are automatically handled.
///     registry.register::<MyComponent>();
/// }
/// world.insert_resource(registry);
/// world.spawn(MyComponent {
///     foo: [1, 2, 3],
///     bar: (1.3, 3.7),
///     baz: String::from("test"),
/// });
///
/// // Print out our serialized scene in the RON format.
/// let registry = world.resource::<AppTypeRegistry>();
/// let scene = DynamicScene::from_world(&world);
/// let scene_serializer = SceneSerializer::new(&scene, &registry.0);
/// println!("{}", bevy_scene::serialize_ron(scene_serializer).unwrap());
/// ```
pub struct SceneSerializer<'a> {
    /// The scene to serialize.
    pub scene: &'a DynamicScene,
    /// Type registry in which the components and resources types used in the scene are registered.
    pub registry: &'a TypeRegistryArc,
}

impl<'a> SceneSerializer<'a> {
    /// Creates a scene serializer.
    pub fn new(scene: &'a DynamicScene, registry: &'a TypeRegistryArc) -> Self {
        SceneSerializer { scene, registry }
    }
}

impl<'a> Serialize for SceneSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(SCENE_STRUCT, 2)?;
        state.serialize_field(
            SCENE_RESOURCES,
            &SceneMapSerializer {
                entries: &self.scene.resources,
                registry: self.registry,
            },
        )?;
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

/// Handles serialization of multiple entities as a map of entity id to serialized entity.
pub struct EntitiesSerializer<'a> {
    /// The entities to serialize.
    pub entities: &'a [DynamicEntity],
    /// Type registry in which the component types used by the entities are registered.
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

/// Handles entity serialization as a map of component type to component value.
pub struct EntitySerializer<'a> {
    /// The entity to serialize.
    pub entity: &'a DynamicEntity,
    /// Type registry in which the component types used by the entity are registered.
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
            &SceneMapSerializer {
                entries: &self.entity.components,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

/// Handles serializing a list of values with a unique type as a map of type to value.
///
/// Used to serialize scene resources in [`SceneSerializer`] and entity components in [`EntitySerializer`].
/// Note that having several entries of the same type in `entries` will lead to an error when using the RON format and
/// deserializing through [`SceneMapDeserializer`].
pub struct SceneMapSerializer<'a> {
    /// List of boxed values of unique type to serialize.
    pub entries: &'a [Box<dyn Reflect>],
    /// Type registry in which the types used in `entries` are registered.
    pub registry: &'a TypeRegistryArc,
}

impl<'a> Serialize for SceneMapSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.entries.len()))?;
        for reflect in self.entries {
            state.serialize_entry(
                reflect.type_name(),
                &TypedReflectSerializer::new(&**reflect, &self.registry.read()),
            )?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum SceneField {
    Resources,
    Entities,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
    Components,
}

/// Handles scene deserialization.
pub struct SceneDeserializer<'a> {
    /// Type registry in which the components and resources types used in the scene to deserialize are registered.
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
            &[SCENE_RESOURCES, SCENE_ENTITIES],
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
        let mut resources = None;
        let mut entities = None;
        while let Some(key) = map.next_key()? {
            match key {
                SceneField::Resources => {
                    if resources.is_some() {
                        return Err(Error::duplicate_field(SCENE_RESOURCES));
                    }
                    resources = Some(map.next_value_seed(SceneMapDeserializer {
                        registry: self.type_registry,
                    })?);
                }
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

        let resources = resources.ok_or_else(|| Error::missing_field(SCENE_RESOURCES))?;
        let entities = entities.ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene {
            resources,
            entities,
        })
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let resources = seq
            .next_element_seed(SceneMapDeserializer {
                registry: self.type_registry,
            })?
            .ok_or_else(|| Error::missing_field(SCENE_RESOURCES))?;

        let entities = seq
            .next_element_seed(SceneEntitiesDeserializer {
                type_registry: self.type_registry,
            })?
            .ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene {
            resources,
            entities,
        })
    }
}

/// Handles deserialization for a collection of entities.
pub struct SceneEntitiesDeserializer<'a> {
    /// Type registry in which the component types used by the entities to deserialize are registered.
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
        while let Some(entity) = map.next_key::<Entity>()? {
            let entity = map.next_value_seed(SceneEntityDeserializer {
                entity,
                type_registry: self.type_registry,
            })?;
            entities.push(entity);
        }

        Ok(entities)
    }
}

/// Handle deserialization of an entity and its components.
pub struct SceneEntityDeserializer<'a> {
    /// Id of the deserialized entity.
    pub entity: Entity,
    /// Type registry in which the component types used by the entity to deserialize are registered.
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
                entity: self.entity,
                registry: self.type_registry,
            },
        )
    }
}

struct SceneEntityVisitor<'a> {
    pub entity: Entity,
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
            .next_element_seed(SceneMapDeserializer {
                registry: self.registry,
            })?
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;

        Ok(DynamicEntity {
            entity: self.entity,
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

                    components = Some(map.next_value_seed(SceneMapDeserializer {
                        registry: self.registry,
                    })?);
                }
            }
        }

        let components = components
            .take()
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
        Ok(DynamicEntity {
            entity: self.entity,
            components,
        })
    }
}

/// Handles deserialization of a sequence of values with unique types.
pub struct SceneMapDeserializer<'a> {
    /// Type registry in which the types of the values to deserialize are registered.
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneMapDeserializer<'a> {
    type Value = Vec<Box<dyn Reflect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(SceneMapVisitor {
            registry: self.registry,
        })
    }
}

struct SceneMapVisitor<'a> {
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SceneMapVisitor<'a> {
    type Value = Vec<Box<dyn Reflect>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("map of reflect types")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut added = HashSet::new();
        let mut entries = Vec::new();
        while let Some(registration) =
            map.next_key_seed(TypeRegistrationDeserializer::new(self.registry))?
        {
            if !added.insert(registration.type_id()) {
                return Err(Error::custom(format_args!(
                    "duplicate reflect type: `{}`",
                    registration.type_name()
                )));
            }

            entries.push(
                map.next_value_seed(TypedReflectDeserializer::new(registration, self.registry))?,
            );
        }

        Ok(entries)
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
    use bevy_ecs::entity::{Entity, EntityMapper, MapEntities};
    use bevy_ecs::prelude::{Component, ReflectComponent, ReflectResource, Resource, World};
    use bevy_ecs::query::{With, Without};
    use bevy_ecs::reflect::{AppTypeRegistry, ReflectMapEntities};
    use bevy_ecs::world::FromWorld;
    use bevy_reflect::{Reflect, ReflectSerialize};
    use bevy_utils::HashMap;
    use bincode::Options;
    use serde::de::DeserializeSeed;
    use serde::Serialize;
    use std::io::BufReader;

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

    #[derive(Reflect, Default)]
    enum MyEnum {
        #[default]
        Unit,
        Tuple(String),
        Struct {
            value: u32,
        },
    }

    #[derive(Resource, Reflect, Default)]
    #[reflect(Resource)]
    struct MyResource {
        foo: i32,
    }

    #[derive(Clone, Component, Reflect, PartialEq)]
    #[reflect(Component, MapEntities, PartialEq)]
    struct MyEntityRef(Entity);

    impl MapEntities for MyEntityRef {
        fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
            self.0 = entity_mapper.get_or_reserve(self.0);
        }
    }

    impl FromWorld for MyEntityRef {
        fn from_world(_world: &mut World) -> Self {
            Self(Entity::PLACEHOLDER)
        }
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
            registry.register::<MyEntityRef>();
            registry.register::<Entity>();
            registry.register::<MyResource>();
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

        world.insert_resource(MyResource { foo: 123 });

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_entities([a, b, c].into_iter());
        builder.extract_resources();
        let scene = builder.build();

        let expected = r#"(
  resources: {
    "bevy_scene::serde::tests::MyResource": (
      foo: 123,
    ),
  },
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
  resources: {
    "bevy_scene::serde::tests::MyResource": (
      foo: 123,
    ),
  },
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
            1,
            scene.resources.len(),
            "expected `resources` to contain 1 resource"
        );
        assert_eq!(
            3,
            scene.entities.len(),
            "expected `entities` to contain 3 entities"
        );

        let mut map = HashMap::default();
        let mut dst_world = create_world();
        scene.write_to_world(&mut dst_world, &mut map).unwrap();

        let my_resource = dst_world.get_resource::<MyResource>();
        assert!(my_resource.is_some());
        let my_resource = my_resource.unwrap();
        assert_eq!(my_resource.foo, 123);

        assert_eq!(3, dst_world.query::<&Foo>().iter(&dst_world).count());
        assert_eq!(2, dst_world.query::<&Bar>().iter(&dst_world).count());
        assert_eq!(1, dst_world.query::<&Baz>().iter(&dst_world).count());
    }

    #[test]
    fn should_roundtrip_with_later_generations_and_obsolete_references() {
        let mut world = create_world();

        world.spawn_empty().despawn();

        let a = world.spawn_empty().id();
        let foo = world.spawn(MyEntityRef(a)).insert(Foo(123)).id();
        world.despawn(a);
        world.spawn(MyEntityRef(foo)).insert(Bar(123));

        let registry = world.resource::<AppTypeRegistry>();

        let scene = DynamicScene::from_world(&world);

        let serialized = scene
            .serialize_ron(&world.resource::<AppTypeRegistry>().0)
            .unwrap();
        let mut deserializer = ron::de::Deserializer::from_str(&serialized).unwrap();
        let scene_deserializer = SceneDeserializer {
            type_registry: &registry.0.read(),
        };

        let deserialized_scene = scene_deserializer.deserialize(&mut deserializer).unwrap();

        let mut map = HashMap::default();
        let mut dst_world = create_world();
        deserialized_scene
            .write_to_world(&mut dst_world, &mut map)
            .unwrap();

        assert_eq!(2, deserialized_scene.entities.len());
        assert_scene_eq(&scene, &deserialized_scene);

        let bar_to_foo = dst_world
            .query_filtered::<&MyEntityRef, Without<Foo>>()
            .get_single(&dst_world)
            .cloned()
            .unwrap();
        let foo = dst_world
            .query_filtered::<Entity, With<Foo>>()
            .get_single(&dst_world)
            .unwrap();

        assert_eq!(foo, bar_to_foo.0);
        assert!(dst_world
            .query_filtered::<&MyEntityRef, With<Foo>>()
            .iter(&dst_world)
            .all(|r| world.get_entity(r.0).is_none()));
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

        let scene = DynamicScene::from_world(&world);

        let scene_serializer = SceneSerializer::new(&scene, &registry.0);
        let serialized_scene = postcard::to_allocvec(&scene_serializer).unwrap();

        assert_eq!(
            vec![
                0, 1, 0, 1, 37, 98, 101, 118, 121, 95, 115, 99, 101, 110, 101, 58, 58, 115, 101,
                114, 100, 101, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77, 121, 67, 111, 109, 112,
                111, 110, 101, 110, 116, 1, 2, 3, 102, 102, 166, 63, 205, 204, 108, 64, 1, 12, 72,
                101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 33
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
    fn should_roundtrip_messagepack() {
        let mut world = create_world();

        world.spawn(MyComponent {
            foo: [1, 2, 3],
            bar: (1.3, 3.7),
            baz: MyEnum::Tuple("Hello World!".to_string()),
        });

        let registry = world.resource::<AppTypeRegistry>();

        let scene = DynamicScene::from_world(&world);

        let scene_serializer = SceneSerializer::new(&scene, &registry.0);
        let mut buf = Vec::new();
        let mut ser = rmp_serde::Serializer::new(&mut buf);
        scene_serializer.serialize(&mut ser).unwrap();

        assert_eq!(
            vec![
                146, 128, 129, 0, 145, 129, 217, 37, 98, 101, 118, 121, 95, 115, 99, 101, 110, 101,
                58, 58, 115, 101, 114, 100, 101, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77, 121,
                67, 111, 109, 112, 111, 110, 101, 110, 116, 147, 147, 1, 2, 3, 146, 202, 63, 166,
                102, 102, 202, 64, 108, 204, 205, 129, 165, 84, 117, 112, 108, 101, 172, 72, 101,
                108, 108, 111, 32, 87, 111, 114, 108, 100, 33
            ],
            buf
        );

        let scene_deserializer = SceneDeserializer {
            type_registry: &registry.0.read(),
        };
        let mut reader = BufReader::new(buf.as_slice());

        let deserialized_scene = scene_deserializer
            .deserialize(&mut rmp_serde::Deserializer::new(&mut reader))
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

        let scene = DynamicScene::from_world(&world);

        let scene_serializer = SceneSerializer::new(&scene, &registry.0);
        let serialized_scene = bincode::serialize(&scene_serializer).unwrap();

        assert_eq!(
            vec![
                0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
                0, 0, 0, 0, 37, 0, 0, 0, 0, 0, 0, 0, 98, 101, 118, 121, 95, 115, 99, 101, 110, 101,
                58, 58, 115, 101, 114, 100, 101, 58, 58, 116, 101, 115, 116, 115, 58, 58, 77, 121,
                67, 111, 109, 112, 111, 110, 101, 110, 116, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0,
                0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 102, 102, 166, 63, 205, 204, 108, 64, 1, 0, 0, 0,
                12, 0, 0, 0, 0, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 33
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
                .unwrap_or_else(|| panic!("missing entity (expected: `{:?}`)", expected.entity));

            assert_eq!(expected.entity, received.entity, "entities did not match");

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
                    "components did not match: (expected: `{expected:?}`, received: `{received:?}`)",
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
            let scene_a = DynamicScene::from_world(&world);

            world.spawn(MyComponent {
                foo: [1, 2, 3],
                bar: (1.3, 3.7),
                baz: MyEnum::Unit,
            });

            let scene_b = DynamicScene::from_world(&world);

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

            let scene_a = DynamicScene::from_world(&world);

            world.entity_mut(entity).insert(MyComponent {
                foo: [3, 2, 1],
                bar: (1.3, 3.7),
                baz: MyEnum::Unit,
            });

            let scene_b = DynamicScene::from_world(&world);

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

            let scene_a = DynamicScene::from_world(&world);

            world.entity_mut(entity).remove::<MyComponent>();

            let scene_b = DynamicScene::from_world(&world);

            assert_scene_eq(&scene_a, &scene_b);
        }
    }
}
