//! Update a scene from a glTF file, either by spawning the scene as a child of another entity,
//! or by accessing the entities of the scene.

use bevy::{prelude::*, scene::SceneInstance};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_scene_entities)
        .run();
}

#[derive(Component)]
struct MovedScene;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..default()
    });
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(1.05, 0.9, 1.5)
            .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..default()
    });

    // Spawn the scene as a child of this entity at the given transform
    commands.spawn_bundle(SceneBundle {
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
        ..default()
    });

    // Spawn a second scene, and add a tag component to be able to target it later
    commands
        .spawn_bundle(SceneBundle {
            scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
            ..default()
        })
        .insert(MovedScene);
}

// This system will move all entities that are in the scene instance of MovedScene
fn move_scene_entities(
    time: Res<Time>,
    moved_scene: Query<&SceneInstance, With<MovedScene>>,
    mut transforms: Query<&mut Transform>,
    scene_spawner: Res<SceneSpawner>,
) {
    // The `SceneInstance` component is added to the scene root once spawning the scene has started
    let scene_root = moved_scene.single();
    let mut offset = 0.0;
    scene_spawner
        .iter_instance_entities(**scene_root)
        .for_each(|entity| {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation = Vec3::new(
                    offset * time.seconds_since_startup().sin() as f32 / 20.,
                    0.,
                    time.seconds_since_startup().cos() as f32 / 20.,
                );
                offset += 1.0;
            }
        });
}
