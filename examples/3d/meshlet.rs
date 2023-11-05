//! GPU-driven meshlet-based rendering.

use bevy::{
    log::info,
    pbr::meshlet::{MaterialMeshletMeshBundle, MeshletMesh, MeshletPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshletPlugin)
        .insert_resource(Msaa::Off) // TODO: MSAA support
        .add_systems(Update, (setup, camera_controller))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshlet_meshes: ResMut<Assets<MeshletMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut dragon_meshlet_mesh_handle: Local<Handle<MeshletMesh>>,
    mut dragon_mesh_handle: Local<Handle<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if dragon_mesh_handle.id() == AssetId::default() {
        commands.spawn((
            Camera3dBundle {
                transform: Transform::from_translation(Vec3::new(1.5658312, 0.3866963, -0.5559159))
                    .with_rotation(Quat::from_array([
                        -0.08115417,
                        0.80237,
                        0.112162426,
                        0.580548,
                    ])),
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            },
            CameraController::default(),
        ));

        commands.spawn(DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_rotation(Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                PI * -0.15,
                PI * -0.15,
            )),
            ..default()
        });

        info!("Loading dragon model...");
        *dragon_mesh_handle = asset_server.load("models/dragon.glb#Mesh0/Primitive0");
    }

    if dragon_meshlet_mesh_handle.id() == AssetId::default() {
        if let Some(dragon_mesh) = meshes.get_mut(&*dragon_mesh_handle) {
            dragon_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[0.0, 0.0]; dragon_mesh.count_vertices()],
            );
            dragon_mesh.generate_tangents().unwrap();

            info!("Calculating dragon meshlets...");
            *dragon_meshlet_mesh_handle =
                meshlet_meshes.add(MeshletMesh::from_mesh(dragon_mesh).unwrap());
            info!("Dragon meshlets calculated");

            commands.spawn(MaterialMeshletMeshBundle {
                meshlet_mesh: dragon_meshlet_mesh_handle.clone(),
                material: materials.add(StandardMaterial {
                    base_color: Color::hex("#ffd891").unwrap(),
                    metallic: 1.0,
                    perceptual_roughness: 0.9,
                    ..default()
                }),
                transform: Transform::default().with_rotation(Quat::from_rotation_x(PI / 2.0)),
                ..default()
            });

            commands.spawn(MaterialMeshletMeshBundle {
                meshlet_mesh: dragon_meshlet_mesh_handle.clone(),
                material: materials.add(StandardMaterial {
                    base_color: Color::hex("#ffc0cb").unwrap(),
                    perceptual_roughness: 0.1,
                    ..default()
                }),
                transform: Transform::default()
                    .with_rotation(Quat::from_rotation_x(PI / 2.0))
                    .with_translation(Vec3::new(1.0, 0.0, 0.0))
                    .with_scale(Vec3::splat(0.5)),
                ..default()
            });
        }
    }
}

// --------------------------------------------------------------------------------------

use bevy::input::mouse::MouseMotion;
use bevy::window::CursorGrabMode;
use std::f32::consts::*;

pub const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

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
