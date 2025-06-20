//! # 2D Point Light Demo
//!
//! Example demonstrating 2D point lights.
//! Features two scenes with different lighting setups and real-time parameter adjustment.
//!
//! Space: Change FallOffType
//! ArrowUp: Increase intensity
//! ArrowDown: Decrease intensity
//! ArrowRight: Cycle through colors
//! N: Next scene
//! O: Disable intensity

use bevy::prelude::*;
use bevy::sprite::{FalloffType, PointLight2D};
use std::f32::consts::TAU;

#[derive(Resource)]
struct LightControl {
    falloff_index: usize,
    color_index: usize,
    last_intensity: f32,
}

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
enum SceneState {
    Scene1,
    Scene2,
}

#[derive(Component)]
struct ControlledLight;

#[derive(Component)]
struct Rotating;

#[derive(Component)]
struct IntensityText;

const FALLOFFS: [FalloffType; 2] = [FalloffType::Linear, FalloffType::Exponential];

const COLORS: [Color; 5] = [
    Color::srgb(1.0, 1.0, 1.0),
    Color::srgb(1.0, 0.0, 0.0),
    Color::srgb(0.0, 1.0, 0.0),
    Color::srgb(0.0, 0.0, 1.0),
    Color::srgb(1.0, 0.5, 0.0),
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "2D Point Light Demo".to_string(),
                resolution: (800., 600.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(LightControl {
            falloff_index: 0,
            color_index: 0,
            last_intensity: 0.0,
        })
        .insert_resource(SceneState::Scene1)
        .add_systems(Startup, setup_scene1)
        .add_systems(Update, (handle_input, switch_scene, rotate_light))
        .run();
}

fn setup_scene1(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2d);

    let sprite_handle = asset_server.load("branding/icon.png");
    let count = 12;
    let radius = 200.0;
    for i in 0..count {
        let angle = i as f32 / count as f32 * TAU;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        commands.spawn((
            Sprite {
                image: sprite_handle.clone(),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0).with_scale(Vec3::splat(0.5)),
        ));
    }

    commands.spawn((
        PointLight2D {
            color: COLORS[0],
            intensity: 1.0,
            radius: 300.0,
            falloff: FALLOFFS[0],
        },
        Transform::from_xyz(0., 0., 1.),
        ControlledLight,
    ));
}

fn setup_scene2(mut commands: Commands, asset_server: Res<AssetServer>) {
    let sprite_handle = asset_server.load("branding/icon.png");

    // Center sprite
    commands.spawn((
        Sprite {
            image: sprite_handle.clone(),
            ..default()
        },
        Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(0.5)),
    ));

    // Rotating light
    commands.spawn((
        PointLight2D {
            color: COLORS[0],
            intensity: 1.0,
            radius: 300.0,
            falloff: FALLOFFS[0],
        },
        Transform::from_xyz(250., 0., 1.),
        ControlledLight,
        Rotating,
    ));
}

fn switch_scene(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scene_state: ResMut<SceneState>,
    asset_server: Res<AssetServer>,
    sprites: Query<Entity, (With<Sprite>, Without<Camera2d>)>,
    lights: Query<Entity, (With<PointLight2D>, Without<Camera2d>)>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyN) {
        for entity in &sprites {
            commands.entity(entity).despawn();
        }

        for entity in &lights {
            commands.entity(entity).despawn();
        }

        *scene_state = match *scene_state {
            SceneState::Scene1 => {
                setup_scene2(commands, asset_server);
                SceneState::Scene2
            }
            SceneState::Scene2 => {
                setup_scene1(commands, asset_server);
                SceneState::Scene1
            }
        };
    }
}

fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut control: ResMut<LightControl>,
    mut query: Query<&mut PointLight2D, With<ControlledLight>>,
) {
    if let Ok(mut light) = query.single_mut() {
        if keyboard_input.just_pressed(KeyCode::Space) {
            control.falloff_index = (control.falloff_index + 1) % FALLOFFS.len();
            light.falloff = FALLOFFS[control.falloff_index];
            info!("Changed falloff to {:?}", light.falloff);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowUp) {
            light.intensity += 0.1;
            info!("Increased intensity: {}", light.intensity);
        }
        if keyboard_input.just_pressed(KeyCode::ArrowDown) {
            light.intensity = (light.intensity - 0.1).max(0.0);
            info!("Decreased intensity: {}", light.intensity);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowRight) {
            control.color_index = (control.color_index + 1) % COLORS.len();
            light.color = COLORS[control.color_index];
            info!("Changed color to {:?}", light.color);
        }

        if keyboard_input.just_pressed(KeyCode::KeyO) {
            if light.intensity > 0.0 {
                control.last_intensity = light.intensity;
                light.intensity = 0.0;
                info!("Light turned off (intensity: 0)");
            } else {
                light.intensity = control.last_intensity;
                info!("Light turned on (intensity: {})", light.intensity);
            }
        }
    }
}

fn rotate_light(time: Res<Time>, mut query: Query<&mut Transform, With<Rotating>>) {
    for mut transform in &mut query {
        let angle = time.delta_secs() * 2.5;
        let current_pos = transform.translation.truncate();
        let rotated_pos = Vec2::from_angle(angle).rotate(current_pos);
        transform.translation = Vec3::new(rotated_pos.x, rotated_pos.y, transform.translation.z);
    }
}
