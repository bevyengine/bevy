//! Demonstrates how to enable per-object motion blur. This rendering feature can be configured per
//! camera using the [`MotionBlur`] component.
//!
//! This example animates some spheres and adds a camera controller to help visualize the effect of
//! the motion blur parameters.

use bevy::core_pipeline::motion_blur::{MotionBlur, MotionBlurBundle};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CameraControllerPlugin))
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(Update, (move_spheres, update_settings))
        .insert_resource(AmbientLight {
            brightness: 0.6,
            ..default()
        })
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(-23.0, 3.0, 1.0))
                .looking_at(Vec3::new(0.0, -2.0, -10.0), Vec3::Y),
            ..default()
        },
        // Adding this bundle to a camera will add motion blur to objects rendered by it.
        MotionBlurBundle {
            // Configure motion blur settings per-camera
            motion_blur: MotionBlur {
                shutter_angle: 1.0,
                max_samples: 8,
                ..default()
            },
            ..default()
        },
        CameraController::default(),
    ));

    // Add a light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 200000.,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ..default()
    });

    let mesh = meshes.add(Mesh::from(shape::UVSphere::default()));
    let image = asset_server.load("textures/checkered.png");
    let mut sphere_matl = |base_color: Color| {
        materials.add(StandardMaterial {
            base_color_texture: Some(image.clone()),
            base_color,
            ..default()
        })
    };
    // Acts like a skybox to allow testing the effects of full screen blur due to camera movement.
    commands.spawn((PbrBundle {
        mesh: mesh.clone(),
        material: sphere_matl(Color::WHITE),
        transform: Transform::from_scale(Vec3::splat(-1000.0)), // In
        ..default()
    },));
    // The rest of the spheres
    commands.spawn((
        PbrBundle {
            mesh: mesh.clone(),
            material: sphere_matl(Color::RED),
            transform: Transform::from_xyz(0.0, 0.7, -7.0),
            ..default()
        },
        Moves,
    ));
    commands.spawn((
        PbrBundle {
            mesh: mesh.clone(),
            material: sphere_matl(Color::GREEN),
            transform: Transform::from_xyz(0.0, -0.8, -2.0),
            ..default()
        },
        Moves,
    ));
    commands.spawn((
        PbrBundle {
            mesh: mesh.clone(),
            material: sphere_matl(Color::CYAN),
            transform: Transform::from_xyz(0.0, 1.8, -4.0),
            ..default()
        },
        Moves,
        Trackable,
    ));
    commands.spawn((
        PbrBundle {
            mesh: mesh.clone(),
            material: sphere_matl(Color::YELLOW),
            transform: Transform::from_xyz(0.0, -1.4, -10.0),
            ..default()
        },
        Moves,
    ));

    commands.spawn((
        PbrBundle {
            mesh: mesh.clone(),
            material: sphere_matl(Color::FUCHSIA),
            transform: Transform::from_xyz(0.0, 0.0, -20.0).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                FRAC_PI_4,
                FRAC_PI_4,
                FRAC_PI_4,
            )),
            ..default()
        },
        Scales,
    ));
}

fn setup_ui(mut commands: Commands) {
    let style = TextStyle {
        font_size: 20.0,
        ..default()
    };

    commands.spawn(
        TextBundle::from_sections(vec![
            TextSection::new(String::new(), style.clone()),
            TextSection::new(String::new(), style.clone()),
            TextSection::new("\n\n", style.clone()),
            TextSection::new("Controls:\n", style.clone()),
            TextSection::new(
                "Spacebar - Toggle camera tracking blue sphere\n",
                style.clone(),
            ),
            TextSection::new("WASD + Mouse - Move camera\n", style.clone()),
            TextSection::new("1/2 - Decrease/Increase shutter angle\n", style.clone()),
            TextSection::new("3/4 - Decrease/Increase sample count\n", style.clone()),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

#[derive(Component)]
struct Moves;

#[derive(Component)]
struct Scales;

#[derive(Component)]
struct Trackable;

/// Rotates any entity around the x and y axis
fn move_spheres(
    time: Res<Time>,
    mut scales: Query<&mut Transform, (With<Scales>, Without<Moves>)>,
    mut moves: Query<&mut Transform, With<Moves>>,
) {
    for mut transform in &mut moves {
        let y = transform.translation.y;
        transform.rotate_x(20. / (y * y) * time.delta_seconds());
        transform.translation.x =
            ((((time.elapsed_seconds() + y) * y * 3.0).sin() * 0.5 + 0.5).powi(4) - 0.5) * 30.0;
    }

    for mut transform in &mut scales {
        transform.scale = Vec3::splat(((time.elapsed_seconds() * 20.0).sin() + 1.0) * 2.0 + 1.0);
    }
}

// Change the intensity over time to show that the effect is controlled from the main world
fn update_settings(
    mut settings: Query<&mut MotionBlur>,
    mut presses: EventReader<bevy::input::keyboard::KeyboardInput>,
    mut text: Query<&mut Text>,
    mut follow: Local<bool>,
    mut camera: Query<&mut Transform, With<Camera>>,
    trackable: Query<&GlobalTransform, With<Trackable>>,
) {
    let mut settings = settings.single_mut();
    for press in presses.read() {
        if press.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if press.key_code == Some(KeyCode::Key1) {
            settings.shutter_angle -= 0.25;
        }
        if press.key_code == Some(KeyCode::Key2) {
            settings.shutter_angle += 0.25;
        }
        if press.key_code == Some(KeyCode::Key3) {
            settings.max_samples -= 1;
        }
        if press.key_code == Some(KeyCode::Key4) {
            settings.max_samples += 1;
        }
        if press.key_code == Some(KeyCode::Space) {
            *follow = !*follow;
        }
        settings.shutter_angle = settings.shutter_angle.clamp(0.0, 100.0);
        settings.max_samples = settings.max_samples.clamp(1, 1000);
    }
    let mut text = text.single_mut();
    text.sections[0].value = format!("Shutter angle: {:.5}\n", settings.shutter_angle);
    text.sections[1].value = format!("Max samples: {:.5}\n", settings.max_samples);

    if *follow {
        let mut camera = camera.single_mut();
        camera.look_at(trackable.single().translation(), Vec3::Y);
    }
}

use bevy::input::mouse::MouseMotion;
use bevy::window::CursorGrabMode;

use std::f32::consts::*;
use std::fmt;

pub const RADIANS_PER_DOT: f32 = 1.0 / 360.0;

#[derive(Component)]
pub struct CameraController {
    pub enabled: bool,
    pub initialized: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub mouse_key_enable_mouse: MouseButton,
    pub keyboard_key_enable_mouse: KeyCode,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub velocity: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            sensitivity: 1.0,
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::ShiftLeft,
            mouse_key_enable_mouse: MouseButton::Left,
            keyboard_key_enable_mouse: KeyCode::M,
            walk_speed: 5.0,
            run_speed: 15.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

impl fmt::Display for CameraController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Freecam Controls:
    MOUSE\t- Move camera orientation
    {:?}/{:?}\t- Enable mouse movement
    {:?}{:?}\t- forward/backward
    {:?}{:?}\t- strafe left/right
    {:?}\t- 'run'
    {:?}\t- up
    {:?}\t- down",
            self.mouse_key_enable_mouse,
            self.keyboard_key_enable_mouse,
            self.key_forward,
            self.key_back,
            self.key_left,
            self.key_right,
            self.key_run,
            self.key_up,
            self.key_down
        )
    }
}

pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, camera_controller);
    }
}

fn camera_controller(
    time: Res<Time>,
    mut windows: Query<&mut Window>,
    mut mouse_events: EventReader<MouseMotion>,
    mouse_button_input: Res<Input<MouseButton>>,
    key_input: Res<Input<KeyCode>>,
    mut move_toggled: Local<bool>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_seconds();

    if let Ok((mut transform, mut options)) = query.get_single_mut() {
        if !options.initialized {
            let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
            options.yaw = yaw;
            options.pitch = pitch;
            options.initialized = true;
        }
        if !options.enabled {
            return;
        }

        // Handle key input
        let mut axis_input = Vec3::ZERO;
        if key_input.pressed(options.key_forward) {
            axis_input.z += 1.0;
        }
        if key_input.pressed(options.key_back) {
            axis_input.z -= 1.0;
        }
        if key_input.pressed(options.key_right) {
            axis_input.x += 1.0;
        }
        if key_input.pressed(options.key_left) {
            axis_input.x -= 1.0;
        }
        if key_input.pressed(options.key_up) {
            axis_input.y += 1.0;
        }
        if key_input.pressed(options.key_down) {
            axis_input.y -= 1.0;
        }
        if key_input.just_pressed(options.keyboard_key_enable_mouse) {
            *move_toggled = !*move_toggled;
        }

        // Apply movement update
        if axis_input != Vec3::ZERO {
            let max_speed = if key_input.pressed(options.key_run) {
                options.run_speed
            } else {
                options.walk_speed
            };
            options.velocity = axis_input.normalize() * max_speed;
        } else {
            let friction = options.friction.clamp(0.0, 1.0);
            options.velocity *= 1.0 - friction;
            if options.velocity.length_squared() < 1e-6 {
                options.velocity = Vec3::ZERO;
            }
        }
        let forward = transform.forward();
        let right = transform.right();
        transform.translation += options.velocity.x * dt * right
            + options.velocity.y * dt * Vec3::Y
            + options.velocity.z * dt * forward;

        // Handle mouse input
        let mut mouse_delta = Vec2::ZERO;
        if mouse_button_input.pressed(options.mouse_key_enable_mouse) || *move_toggled {
            for mut window in &mut windows {
                if !window.focused {
                    continue;
                }

                window.cursor.grab_mode = CursorGrabMode::Locked;
                window.cursor.visible = false;
            }

            for mouse_event in mouse_events.read() {
                mouse_delta += mouse_event.delta;
            }
        }
        if mouse_button_input.just_released(options.mouse_key_enable_mouse) {
            for mut window in &mut windows {
                window.cursor.grab_mode = CursorGrabMode::None;
                window.cursor.visible = true;
            }
        }

        if mouse_delta != Vec2::ZERO {
            // Apply look update
            options.pitch = (options.pitch - mouse_delta.y * RADIANS_PER_DOT * options.sensitivity)
                .clamp(-PI / 2., PI / 2.);
            options.yaw -= mouse_delta.x * RADIANS_PER_DOT * options.sensitivity;
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, options.yaw, options.pitch);
        }
    }
}
