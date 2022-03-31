use bevy::{prelude::*, scene::InstanceId};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0,
        })
        .add_startup_system(setup)
        .add_system(setup_scene_once_loaded)
        .add_system(keyboard_animation_control)
        .run();
}

struct CurrentScene {
    instance_id: InstanceId,
    animation: Handle<AnimationClip>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scene_spawner: ResMut<SceneSpawner>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Insert a resource with the current scene information
    commands.insert_resource(CurrentScene {
        // Its instance id, to be able to check that it's loaded
        instance_id: scene_spawner.spawn(asset_server.load("models/animated/Fox.glb#Scene0")),
        // The handle to the run animation
        animation: asset_server.load("models/animated/Fox.glb#Animation2"),
    });

    // Camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(100.0, 100.0, 150.0)
            .looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    // Plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 500000.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // Light
    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            1.0,
            -std::f32::consts::FRAC_PI_4,
        )),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    println!("Animation controls:");
    println!("  - spacebar: play / pause");
    println!("  - arrow up / down: speed up / slow down animation playback");
    println!("  - arrow left / right: seek backward / forward");
}

// Setup the scene for animation once it is loaded, by adding the animation to the root entity of
// the scene.
fn setup_scene_once_loaded(
    mut commands: Commands,
    scene_spawner: Res<SceneSpawner>,
    current_scene: Res<CurrentScene>,
    named: Query<&Name>,
    mut done: Local<bool>,
) {
    // Once the scene is loaded, start the animation
    if !*done {
        if let Some(mut entity_iter) =
            scene_spawner.iter_instance_entities(current_scene.instance_id)
        {
            // Find the root entity for the loaded scene. The name is set from the gltf file.
            let root = Name::new("root".to_string());
            if let Some(root) = entity_iter.find(|entity| {
                if let Ok(name) = named.get(*entity) {
                    if *name == root {
                        return true;
                    }
                }
                false
            }) {
                // Insert the handle to the animation on the root entity.
                commands.entity(root).insert_bundle(AnimationBundle {
                    handle: current_scene.animation.clone_weak(),
                    player: AnimationPlayer {
                        looping: true,
                        ..default()
                    },
                });
            }
            *done = true;
        }
    }
}

fn keyboard_animation_control(
    keyboard_input: Res<Input<KeyCode>>,
    mut animation_player: Query<&mut AnimationPlayer>,
) {
    for mut player in animation_player.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::Space) {
            player.paused = !player.paused;
        }

        if keyboard_input.just_pressed(KeyCode::Up) {
            player.speed *= 1.2;
        }

        if keyboard_input.just_pressed(KeyCode::Down) {
            player.speed *= 0.8;
        }

        if keyboard_input.just_pressed(KeyCode::Left) {
            player.elapsed = (player.elapsed - 0.1).max(0.0);
        }

        if keyboard_input.just_pressed(KeyCode::Right) {
            player.elapsed += 0.1;
        }
    }
}
