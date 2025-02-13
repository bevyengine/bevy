//! Loads and renders a glTF file as a scene, and adds the different `gltf_extras` as Components
//! deserialized from the JSON.

use std::fmt::Debug;

use bevy::{
    ecs::reflect::ReflectCommandExt,
    gltf::{GltfExtras, GltfMaterialExtras, GltfMeshExtras, GltfSceneExtras},
    prelude::*,
    reflect::{serde::ReflectDeserializer, TypeRegistry},
    scene::SceneInstanceReady,
};
use serde::{de::DeserializeSeed, Deserialize, Serialize};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_type::<MyMaterialProperty>()
        .register_type::<MySceneProperty>()
        .register_type::<MyCustomObjectProperty>()
        .register_type::<SomeCustomMeshProperty>()
        .add_systems(Startup, setup)
        .add_systems(Update, check_for_gltf_components)
        .add_observer(add_components_from_gltf_extras)
        .run();
}

#[derive(Component)]
struct ExampleDisplay;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn(DirectionalLight {
        shadows_enabled: true,
        ..default()
    });

    // a barebones scene containing one of each gltf_extra type
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/extras/gltf_extras.glb"),
    )));

    // a place to display the extras on screen
    commands.spawn((
        Text::default(),
        TextFont {
            font_size: 15.,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ExampleDisplay,
    ));
}

/// This system just checks for the custom `Component`s that were added to the entities loaded from
/// the gltf file.
fn check_for_gltf_components(
    objects: Query<(Entity, Option<&Name>, &MyCustomObjectProperty)>,
    scenes: Query<(Entity, Option<&Name>, &MySceneProperty)>,
    materials: Query<(Entity, Option<&Name>, &MyMaterialProperty)>,
    meshes: Query<(Entity, Option<&Name>, &SomeCustomMeshProperty)>,
    mut display: Single<&mut Text, With<ExampleDisplay>>,
) {
    let mut gltf_extra_infos_lines: Vec<String> = vec![];
    for (id, name, object) in objects.iter() {
        gltf_extra_infos_lines.push(generate_entity_info(id, name, object));
    }
    for (id, name, scene) in scenes.iter() {
        gltf_extra_infos_lines.push(generate_entity_info(id, name, scene));
    }
    for (id, name, material) in materials.iter() {
        gltf_extra_infos_lines.push(generate_entity_info(id, name, material));
    }
    for (id, name, mesh) in meshes.iter() {
        gltf_extra_infos_lines.push(generate_entity_info(id, name, mesh));
    }
    display.0 = gltf_extra_infos_lines.join("\n");
}

/// Helper function to generate a string from the entity info.
fn generate_entity_info(id: Entity, name: Option<&Name>, display: impl Debug) -> String {
    let formatted_extras = format!(
        "Extras per entity {} ('Name: {}'):
    - component: {:?}
                ",
        id,
        name.unwrap_or(&Name::default()),
        display,
    );
    formatted_extras
}

/// An observer that adds the custom `Component`s to the entities that have
/// `GltfExtras`, `GltfSceneExtras`, `GltfMaterialExtras`, or `GltfMeshExtras`.
///
/// This system runs as soon as the gltf scene is ready, and the `SceneInstanceReady` trigger is
/// fired.
fn add_components_from_gltf_extras(
    trigger: Trigger<SceneInstanceReady>,
    type_registry: Res<AppTypeRegistry>,
    children: Query<&Children>,
    primitives: Query<&GltfExtras>,
    scenes: Query<&GltfSceneExtras>,
    materials: Query<&GltfMaterialExtras>,
    meshes: Query<&GltfMeshExtras>,
    mut commands: Commands,
) {
    let type_registry = type_registry.read();

    for child in children.iter_descendants(trigger.target()) {
        if let Some(extras) = primitives.get(child).ok() {
            info!("Found extras on {child:?}: {extras:?}");
            let value = deserialize_extra(&type_registry, &extras.value);
            commands.entity(child).insert_reflect(value);
        }
        if let Some(extras) = scenes.get(child).ok() {
            info!("Found extras on {child:?}: {extras:?}");
            let value = deserialize_extra(&type_registry, &extras.value);
            commands.entity(child).insert_reflect(value);
        }
        if let Some(extras) = materials.get(child).ok() {
            info!("Found extras on {child:?}: {extras:?}");
            let value = deserialize_extra(&type_registry, &extras.value);
            commands.entity(child).insert_reflect(value);
        }
        if let Some(extras) = meshes.get(child).ok() {
            info!("Found extras on {child:?}: {extras:?}");
            let value = deserialize_extra(&type_registry, &extras.value);
            commands.entity(child).insert_reflect(value);
        }
    }
}

/// Deserialize the `Component` from the JSON data stored in the `&str`.
fn deserialize_extra(type_registry: &TypeRegistry, extras: &str) -> Box<dyn PartialReflect> {
    let reflect_deserializer = ReflectDeserializer::new(type_registry);
    let mut deserializer = serde_json::Deserializer::from_str(extras);
    let value: Box<dyn PartialReflect> =
        reflect_deserializer.deserialize(&mut deserializer).unwrap();
    info!("Deserialized Component: {value:?}");
    value
}

/// This `Component` is deserialized from the JSON data stored in the `GltfMaterialExtras`.
#[derive(Component, Reflect, Debug, Default, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize, Debug, type_path = false)]
struct MyMaterialProperty(String);

/// Implementing `TypePath` allows the `TypeRegistry` to know the type path of the component.
/// We are implementing it ourselves, because the JSON data stored in the gltf doesn't match the
/// default serialization format that bevy uses.
///
/// This implementation allows the following JSON object to be deserialized correctly:
/// ```json
/// {"my_material_property": "some string"}
/// ```
///
/// The default implementation would expect the following JSON object:
/// ```json
/// {"load_gltf_components::MyMaterialProperty": "some string"}
/// ```
///
/// Both are valid, but in order to keep with the existing JSON data, we've implemented `TypePath`
/// ourselves.
impl TypePath for MyMaterialProperty {
    fn type_path() -> &'static str {
        "my_material_property"
    }

    fn short_type_path() -> &'static str {
        "my_material_property"
    }
}

/// This `Component` is deserialized from the JSON data stored in the `GltfSceneExtras`.
#[derive(Component, Reflect, Debug, Default, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize, Debug, type_path = false)]
struct MySceneProperty(i64);

/// This is the same reasoning as for `MyMaterialProperty`'s `TypePath` implementation.
impl TypePath for MySceneProperty {
    fn type_path() -> &'static str {
        "my_scene_property"
    }

    fn short_type_path() -> &'static str {
        "my_scene_property"
    }
}

/// This `Component` is deserialized from the JSON data stored in the `GltfExtras`.
#[derive(Component, Reflect, Debug, Default, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize, Debug, type_path = false)]
struct MyCustomObjectProperty(usize);

/// This is the same reasoning as for `MyMaterialProperty`'s `TypePath` implementation.
impl TypePath for MyCustomObjectProperty {
    fn type_path() -> &'static str {
        "my_custom_object_property"
    }

    fn short_type_path() -> &'static str {
        "my_custom_object_property"
    }
}

/// This `Component` is deserialized from the JSON data stored in the `GltfMeshExtras`.
#[derive(Component, Reflect, Debug, Default, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize, Debug, type_path = false)]
struct SomeCustomMeshProperty(f32);

/// This is the same reasoning as for `MyMaterialProperty`'s `TypePath` implementation.
impl TypePath for SomeCustomMeshProperty {
    fn type_path() -> &'static str {
        "some_custom_mesh_property"
    }

    fn short_type_path() -> &'static str {
        "some_custom_mesh_property"
    }
}
