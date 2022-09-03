//! Edit a scene from a glTF file, before spawning the scene in the world

use bevy::{prelude::*, reflect::TypeUuid};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(setup_scenes)
        .run();
}

pub const CAKE_WITH_LIGHT_HANDLE: HandleUntyped = HandleUntyped::weak_from_u64(Scene::TYPE_UUID, 1);
pub const CAKE_WITH_1_SLICE_MISSING_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Scene::TYPE_UUID, 2);

#[derive(Resource, Deref)]
struct OriginalScene(Handle<Scene>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(2.0, 1.0, 0.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..default()
    });

    commands.insert_resource(OriginalScene(
        asset_server.load("models/AlienCake/cakeBirthday.glb#Scene0"),
    ));

    commands.spawn_bundle(SceneBundle {
        scene: CAKE_WITH_LIGHT_HANDLE.typed(),
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        ..default()
    });

    commands.spawn_bundle(SceneBundle {
        scene: CAKE_WITH_1_SLICE_MISSING_HANDLE.typed(),
        transform: Transform::from_xyz(0.0, 0.0, 1.0),
        ..default()
    });
}

fn setup_scenes(
    mut scenes: ResMut<Assets<Scene>>,
    original_scene: Res<OriginalScene>,
    type_registry: Res<AppTypeRegistry>,
    mut done: Local<bool>,
) {
    if !*done {
        if let Some(original_scene) = scenes.get(&*original_scene) {
            let slice_name = Name::new("slice");

            // add lights to the original scene
            let mut scene_with_lights = original_scene.clone_with(&*type_registry).unwrap();
            let mut query = scene_with_lights.query::<(Entity, &Name)>();
            let slice_entities = query
                .iter(&scene_with_lights)
                .filter(|(_, name)| **name == slice_name)
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>();
            for slice_entity in &slice_entities {
                scene_with_lights
                    .entity_mut(*slice_entity)
                    .with_children(|builder| {
                        builder.spawn_bundle(PointLightBundle {
                            point_light: PointLight {
                                intensity: 1.0,
                                range: 0.3,
                                shadows_enabled: true,
                                ..default()
                            },
                            // This transform is at the flame of the candle
                            transform: Transform::from_xyz(0.05, 0.35, 0.0),
                            ..default()
                        });
                    });
            }

            // remove one slice from the original scene
            let mut scene_with_1_slice_missing =
                original_scene.clone_with(&*type_registry).unwrap();
            let mut query = scene_with_1_slice_missing.query::<(Entity, &Name)>();
            let first_slice_entity = query
                .iter(&scene_with_1_slice_missing)
                .filter(|(_, name)| **name == slice_name)
                .map(|(entity, _)| entity)
                .next()
                .unwrap();
            scene_with_1_slice_missing
                .entity_mut(first_slice_entity)
                .despawn_recursive();

            // add the modified scenes to the assets
            scenes.set_untracked(CAKE_WITH_LIGHT_HANDLE, scene_with_lights);
            scenes.set_untracked(CAKE_WITH_1_SLICE_MISSING_HANDLE, scene_with_1_slice_missing);

            *done = true;
        }
    }
}
