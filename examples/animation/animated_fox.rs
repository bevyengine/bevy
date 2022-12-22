//! Plays animations from a skinned glTF.

use std::f32::consts::PI;
use std::time::Duration;

use bevy::{prelude::*, render::picking::Picking, scene::SceneInstance, utils::HashSet};

#[derive(Debug, Default, Deref, DerefMut, Resource)]
struct FoxEntities(HashSet<Entity>);

const LIGHT_FOX_PICKED: Color = Color::WHITE;
const LIGHT_FOX_UNPICKED: Color = Color::CRIMSON;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0,
        })
        .init_resource::<FoxEntities>()
        .add_startup_system(setup)
        .add_system(setup_scene_once_loaded)
        .add_system(keyboard_animation_control)
        .add_system(store_fox_entites)
        .add_system(picking)
        .run();
}

#[derive(Resource)]
struct Animations(Vec<Handle<AnimationClip>>);

#[derive(Component)]
struct Fox;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Insert a resource with the current scene information
    commands.insert_resource(Animations(vec![
        asset_server.load("models/animated/Fox.glb#Animation2"),
        asset_server.load("models/animated/Fox.glb#Animation1"),
        asset_server.load("models/animated/Fox.glb#Animation0"),
    ]));

    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(100.0, 100.0, 150.0)
                .looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
            ..default()
        },
        Picking::default(),
    ));

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 500000.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            color: LIGHT_FOX_UNPICKED,
            ..default()
        },
        ..default()
    });

    // Debug text
    commands.spawn(
        TextBundle::from_section(
            "",
            TextStyle {
                // TODO: Mono
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 50.0,
                color: Color::WHITE,
            },
        )
        .with_text_alignment(TextAlignment::CENTER)
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                right: Val::Px(15.0),
                ..default()
            },
            max_size: Size {
                width: Val::Px(400.),
                height: Val::Undefined,
            },
            ..default()
        }),
    );

    commands.spawn((
        SceneBundle {
            scene: asset_server.load("models/animated/Fox.glb#Scene0"),
            ..default()
        },
        Fox,
    ));

    println!("Animation controls:");
    println!("  - spacebar: play / pause");
    println!("  - arrow up / down: speed up / slow down animation playback");
    println!("  - arrow left / right: seek backward / forward");
    println!("  - return: change animation");
}

// Once the scene is loaded, start the animation
fn setup_scene_once_loaded(
    animations: Res<Animations>,
    mut player: Query<&mut AnimationPlayer>,
    mut done: Local<bool>,
) {
    if !*done {
        if let Ok(mut player) = player.get_single_mut() {
            player.play(animations.0[0].clone_weak()).repeat();
            *done = true;
        }
    }
}

fn keyboard_animation_control(
    keyboard_input: Res<Input<KeyCode>>,
    mut animation_player: Query<&mut AnimationPlayer>,
    animations: Res<Animations>,
    mut current_animation: Local<usize>,
) {
    if let Ok(mut player) = animation_player.get_single_mut() {
        if keyboard_input.just_pressed(KeyCode::Space) {
            if player.is_paused() {
                player.resume();
            } else {
                player.pause();
            }
        }

        if keyboard_input.just_pressed(KeyCode::Up) {
            let speed = player.speed();
            player.set_speed(speed * 1.2);
        }

        if keyboard_input.just_pressed(KeyCode::Down) {
            let speed = player.speed();
            player.set_speed(speed * 0.8);
        }

        if keyboard_input.just_pressed(KeyCode::Left) {
            let elapsed = player.elapsed();
            player.set_elapsed(elapsed - 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Right) {
            let elapsed = player.elapsed();
            player.set_elapsed(elapsed + 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Return) {
            *current_animation = (*current_animation + 1) % animations.0.len();
            player
                .play_with_transition(
                    animations.0[*current_animation].clone_weak(),
                    Duration::from_millis(250),
                )
                .repeat();
        }
    }
}

#[derive(Default, Deref, DerefMut)]
struct Done(bool);

// In order to know when the fox is picked, we need to store which entites
// belong to it.
fn store_fox_entites(
    mut done: Local<Done>,
    mut fox_entities: ResMut<FoxEntities>,
    scene_spawner: Res<SceneSpawner>,
    fox_scene: Query<&SceneInstance, With<Fox>>,
) {
    if let Ok(id) = fox_scene.get_single() {
        if !**done && scene_spawner.instance_is_ready(**id) {
            **fox_entities = HashSet::from_iter(scene_spawner.iter_instance_entities(**id));
            **done = true;
        }
    }
}

fn picking(
    fox_entities: Res<FoxEntities>,
    mut light: Query<&mut DirectionalLight>,
    mut text: Query<&mut Text>,
    mut cursor_moved: EventReader<CursorMoved>,
    picking_camera: Query<(&Picking, &Camera)>,
) {
    if fox_entities.is_empty() {
        // Not ready
        return;
    }

    let (picking, camera) = picking_camera.single();

    for moved in cursor_moved.iter() {
        let coordinates = moved.position.as_uvec2();

        let depth = picking.depth(camera, coordinates);

        text.single_mut().sections[0].value = format!("Depth: {depth:.7?}");

        if let Some(entity) = picking.get_entity(camera, coordinates) {
            if fox_entities.contains(&entity) {
                let mut light = light.single_mut();
                light.color = LIGHT_FOX_PICKED;
            } else {
                let mut light = light.single_mut();
                light.color = LIGHT_FOX_UNPICKED;
            }
        }
    }
}
