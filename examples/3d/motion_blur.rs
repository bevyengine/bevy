//! Demonstrates how to enable per-object motion blur. This rendering feature can be configured per
//! camera using the [`MotionBlur`] component.
//!
//! This example animates some meshes and adds a camera controller to help visualize the effect of
//! the motion blur parameters.

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        fxaa::Fxaa,
        motion_blur::{MotionBlur, MotionBlurBundle},
        tonemapping::Tonemapping,
    },
    input::mouse::MouseMotion,
    pbr::NotShadowReceiver,
    prelude::*,
    window::CursorGrabMode,
};
use bevy_internal::prelude::shape::UVSphere;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CameraControllerPlugin,
            bevy_internal::core_pipeline::experimental::taa::TemporalAntiAliasPlugin,
        ))
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(Update, (translate, rotate, scale, update_settings).chain())
        .insert_resource(Msaa::Off) //
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    #[allow(clippy::needless_update)]
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-35.0, 8.0, 45.0))
                .looking_at(Vec3::new(0.0, -4.0, 20.0), Vec3::Y),
            tonemapping: Tonemapping::TonyMcMapface,
            ..default()
        },
        // Adding this bundle to a camera will add motion blur to objects rendered by it.
        MotionBlurBundle {
            // Configure motion blur settings per-camera
            motion_blur: MotionBlur {
                shutter_angle: 0.5,
                max_samples: 8,
                ..default()
            },
            ..default()
        },
        CameraController::default(),
        Fxaa::default(),
        BloomSettings::default(),
    ));

    // Add a light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 30000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_translation(Vec3::Y * 100.0).looking_at(Vec3::X, Vec3::Y),
        ..default()
    });

    let sphere = meshes.add(Mesh::from(shape::UVSphere {
        radius: 1.0,
        ..default()
    }));

    let image = asset_server.load("textures/checkered.png");
    // Acts like a skybox to allow testing the effects of full screen blur due to camera movement.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere::default())),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.1, 0.1, 0.1),
                perceptual_roughness: 1.0,
                reflectance: 0.0,
                base_color_texture: Some(image.clone()),
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(-1000.0))
                .with_translation(Vec3::Y * -200.0),
            ..default()
        },
        NotShadowReceiver,
    ));
    commands.spawn((PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane {
            size: 500.0,
            subdivisions: 10,
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::DARK_GRAY,
            base_color_texture: Some(image.clone()),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -10.0, 0.0),
        ..default()
    },));
    // The rest of the spheres
    let mut sphere_matl = |base_color: Color| {
        materials.add(StandardMaterial {
            base_color_texture: Some(image.clone()),
            base_color,
            perceptual_roughness: 0.2,
            ..default()
        })
    };
    commands.spawn((
        PbrBundle {
            mesh: sphere.clone(),
            material: sphere_matl(Color::BLUE),
            transform: Transform::from_xyz(0.0, 0.0, 40.0),
            ..default()
        },
        Translates(0.8),
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere.clone(),
            material: sphere_matl(Color::GREEN),
            transform: Transform::from_xyz(0.0, 0.0, 30.0),
            ..default()
        },
        Translates(1.0),
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere.clone(),
            material: sphere_matl(Color::RED),
            transform: Transform::from_xyz(0.0, 0.0, 20.0),
            ..default()
        },
        Translates(1.2),
        Trackable,
    ));
    commands.spawn((
        PbrBundle {
            mesh: sphere.clone(),
            material: sphere_matl(Color::YELLOW),
            transform: Transform::from_xyz(0.0, 0.0, 10.0),
            ..default()
        },
        Translates(1.4),
    ));

    commands.spawn((
        PbrBundle {
            mesh: sphere.clone(),
            material: sphere_matl(Color::FUCHSIA),
            transform: Transform::from_xyz(0.0, 0.0, -30.0).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                FRAC_PI_4,
                -FRAC_PI_4,
                FRAC_PI_4,
            )),
            ..default()
        },
        Scales(20.0),
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(UVSphere::default().into()),
            material: sphere_matl(Color::WHITE),
            transform: Transform::from_xyz(100.0, 50.0, -100.0)
                .with_scale(Vec3::splat(60.0))
                .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
            ..default()
        },
        Rotates(30.0),
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
            TextSection::new(String::new(), style.clone()),
            TextSection::new("\n\n", style.clone()),
            TextSection::new("Controls:\n", style.clone()),
            TextSection::new("Spacebar - Toggle camera tracking\n", style.clone()),
            TextSection::new("WASD + Mouse - Move camera\n", style.clone()),
            TextSection::new("1/2 - Decrease/Increase shutter angle\n", style.clone()),
            TextSection::new("3/4 - Decrease/Increase sample count\n", style.clone()),
            TextSection::new("5/6 - Decrease/Increase depth bias\n", style.clone()),
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
struct Translates(f32);

#[derive(Component)]
struct Scales(f32);

#[derive(Component)]
struct Rotates(f32);

#[derive(Component)]
struct Trackable;

fn translate(time: Res<Time>, mut moves: Query<(&mut Transform, &Translates)>) {
    for (mut transform, moves) in &mut moves {
        let t = time.elapsed_seconds();
        transform.translation.x =
            ((((t + moves.0) * moves.0 * 3.).sin() * 0.5 + 0.5).powi(4) - 0.5) * 30.;
    }
}

fn rotate(time: Res<Time>, mut moves: Query<(&mut Transform, &Rotates)>) {
    for (mut transform, rotate) in &mut moves {
        transform.rotate_local_z(rotate.0 * time.delta_seconds());
    }
}

fn scale(time: Res<Time>, mut moves: Query<(&mut Transform, &Scales)>) {
    for (mut transform, scales) in &mut moves {
        transform.scale =
            Vec3::splat(((time.elapsed_seconds() * scales.0).sin() + 1.0) * 2.0 + 1.0);
    }
}

// Change the intensity over time to show that the effect is controlled from the main world
fn update_settings(
    mut settings: Query<&mut MotionBlur>,
    mut presses: EventReader<bevy::input::keyboard::KeyboardInput>,
    mut text: Query<&mut Text>,
    mut follow: Local<bool>,
    mut camera: Query<&mut Transform, With<Camera>>,
    trackable: Query<&Transform, (With<Trackable>, Without<Camera>)>,
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
        camera.look_at(trackable.single().translation, Vec3::Y);
    }
}

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
            walk_speed: 20.0,
            run_speed: 100.0,
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
        app.add_systems(PreUpdate, camera_controller);
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
