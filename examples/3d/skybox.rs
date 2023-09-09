//! Load a cubemap texture onto a cube like a skybox and cycle through different compressed texture formats

use bevy::{
    asset::LoadState,
    core_pipeline::Skybox,
    input::mouse::MouseMotion,
    prelude::*,
    render::{
        render_resource::{TextureViewDescriptor, TextureViewDimension},
        renderer::RenderDevice,
        texture::CompressedImageFormats,
    },
};
use std::f32::consts::PI;

const CUBEMAPS: &[(&str, CompressedImageFormats)] = &[
    (
        "textures/Ryfjallet_cubemap.png",
        CompressedImageFormats::NONE,
    ),
    (
        "textures/Ryfjallet_cubemap_astc4x4.ktx2",
        CompressedImageFormats::ASTC_LDR,
    ),
    (
        "textures/Ryfjallet_cubemap_bc7.ktx2",
        CompressedImageFormats::BC,
    ),
    (
        "textures/Ryfjallet_cubemap_etc2.ktx2",
        CompressedImageFormats::ETC2,
    ),
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                cycle_cubemap_asset,
                asset_loaded.after(cycle_cubemap_asset),
                camera_controller,
                animate_light_direction,
            ),
        )
        .run();
}

#[derive(Resource)]
struct Cubemap {
    is_loaded: bool,
    index: usize,
    image_handle: Handle<Image>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // directional 'sun' light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 32000.0,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 2.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
        ..default()
    });

    let skybox_handle = asset_server.load(CUBEMAPS[0].0);
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        CameraController::default(),
        Skybox(skybox_handle.clone()),
    ));

    // ambient light
    // NOTE: The ambient light is used to scale how bright the environment map is so with a bright
    // environment map, use an appropriate color and brightness to match
    commands.insert_resource(AmbientLight {
        color: Color::rgb_u8(210, 220, 240),
        brightness: 1.0,
    });

    commands.insert_resource(Cubemap {
        is_loaded: false,
        index: 0,
        image_handle: skybox_handle,
    });
}

const CUBEMAP_SWAP_DELAY: f32 = 3.0;

fn cycle_cubemap_asset(
    time: Res<Time>,
    mut next_swap: Local<f32>,
    mut cubemap: ResMut<Cubemap>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    let now = time.elapsed_seconds();
    if *next_swap == 0.0 {
        *next_swap = now + CUBEMAP_SWAP_DELAY;
        return;
    } else if now < *next_swap {
        return;
    }
    *next_swap += CUBEMAP_SWAP_DELAY;

    let supported_compressed_formats =
        CompressedImageFormats::from_features(render_device.features());

    let mut new_index = cubemap.index;
    for _ in 0..CUBEMAPS.len() {
        new_index = (new_index + 1) % CUBEMAPS.len();
        if supported_compressed_formats.contains(CUBEMAPS[new_index].1) {
            break;
        }
        info!("Skipping unsupported format: {:?}", CUBEMAPS[new_index]);
    }

    // Skip swapping to the same texture. Useful for when ktx2, zstd, or compressed texture support
    // is missing
    if new_index == cubemap.index {
        return;
    }

    cubemap.index = new_index;
    cubemap.image_handle = asset_server.load(CUBEMAPS[cubemap.index].0);
    cubemap.is_loaded = false;
}

fn asset_loaded(
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut cubemap: ResMut<Cubemap>,
    mut skyboxes: Query<&mut Skybox>,
) {
    if !cubemap.is_loaded && asset_server.load_state(&cubemap.image_handle) == LoadState::Loaded {
        info!("Swapping to {}...", CUBEMAPS[cubemap.index].0);
        let image = images.get_mut(&cubemap.image_handle).unwrap();
        // NOTE: PNGs do not have any metadata that could indicate they contain a cubemap texture,
        // so they appear as one texture. The following code reconfigures the texture as necessary.
        if image.texture_descriptor.array_layer_count() == 1 {
            image.reinterpret_stacked_2d_as_array(
                image.texture_descriptor.size.height / image.texture_descriptor.size.width,
            );
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
        }

        for mut skybox in &mut skyboxes {
            skybox.0 = cubemap.image_handle.clone();
        }

        cubemap.is_loaded = true;
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() * 0.5);
    }
}

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
            sensitivity: 0.5,
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::ShiftLeft,
            mouse_key_enable_mouse: MouseButton::Left,
            keyboard_key_enable_mouse: KeyCode::M,
            walk_speed: 2.0,
            run_speed: 6.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

pub fn camera_controller(
    time: Res<Time>,
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
            for mouse_event in mouse_events.read() {
                mouse_delta += mouse_event.delta;
            }
        }

        if mouse_delta != Vec2::ZERO {
            // Apply look update
            options.pitch = (options.pitch - mouse_delta.y * 0.5 * options.sensitivity * dt)
                .clamp(-PI / 2., PI / 2.);
            options.yaw -= mouse_delta.x * options.sensitivity * dt;
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, options.yaw, options.pitch);
        }
    }
}
