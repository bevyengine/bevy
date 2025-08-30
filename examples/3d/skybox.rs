//! Load a cubemap texture onto a cube like a skybox and cycle through different compressed texture formats

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    anti_aliasing::taa::TemporalAntiAliasing, core_pipeline::Skybox, image::CompressedImageFormats,
    pbr::ScreenSpaceAmbientOcclusion, prelude::*, render::renderer::RenderDevice,
};
use bevy_image::{ImageLoaderSettings, ImageTextureViewDimension};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

const CUBEMAPS: &[(
    &str,
    CompressedImageFormats,
    Option<ImageTextureViewDimension>,
)] = &[
    (
        "textures/Ryfjallet_cubemap.png",
        CompressedImageFormats::NONE,
        // NOTE: PNGs do not have any metadata that could indicate they contain a cubemap texture,
        // so they appear as one texture. This is passed to ImageLoaderSettings to reconfigure the texture as necessary during load.
        Some(ImageTextureViewDimension::Cube),
    ),
    (
        "textures/Ryfjallet_cubemap_astc4x4.ktx2",
        CompressedImageFormats::ASTC_LDR,
        None,
    ),
    (
        "textures/Ryfjallet_cubemap_bc7.ktx2",
        CompressedImageFormats::BC,
        None,
    ),
    (
        "textures/Ryfjallet_cubemap_etc2.ktx2",
        CompressedImageFormats::ETC2,
        None,
    ),
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (cycle_cubemap_asset, animate_light_direction))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // directional 'sun' light
    commands.spawn((
        DirectionalLight {
            illuminance: 32000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.0, 0.0).with_rotation(Quat::from_rotation_x(-PI / 4.)),
    ));

    let skybox_handle =
        asset_server.load_with_settings(CUBEMAPS[0].0, |settings: &mut ImageLoaderSettings| {
            settings.view_dimension = Some(ImageTextureViewDimension::Cube);
        });

    // camera
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        TemporalAntiAliasing::default(),
        ScreenSpaceAmbientOcclusion::default(),
        Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraController::default(),
        Skybox {
            image: skybox_handle.clone(),
            brightness: 1000.0,
            ..default()
        },
    ));

    // ambient light
    // NOTE: The ambient light is used to scale how bright the environment map is so with a bright
    // environment map, use an appropriate color and brightness to match
    commands.insert_resource(AmbientLight {
        color: Color::srgb_u8(210, 220, 240),
        brightness: 1.0,
        ..default()
    });
}

const CUBEMAP_SWAP_DELAY: f32 = 3.0;

fn cycle_cubemap_asset(
    time: Res<Time>,
    mut next_swap: Local<f32>,
    mut index: Local<usize>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
    mut skyboxes: Query<&mut Skybox>,
) {
    let now = time.elapsed_secs();
    if *next_swap == 0.0 {
        *next_swap = now + CUBEMAP_SWAP_DELAY;
        return;
    } else if now < *next_swap {
        return;
    }
    *next_swap += CUBEMAP_SWAP_DELAY;

    let supported_compressed_formats =
        CompressedImageFormats::from_features(render_device.features());

    let mut new_index = *index;
    for _ in 0..CUBEMAPS.len() {
        new_index = (new_index + 1) % CUBEMAPS.len();
        if supported_compressed_formats.contains(CUBEMAPS[new_index].1) {
            break;
        }
        info!(
            "Skipping format which is not supported by current hardware: {:?}",
            CUBEMAPS[new_index]
        );
    }

    // Skip swapping to the same texture. Useful for when ktx2, zstd, or compressed texture support
    // is missing
    if new_index == *index {
        return;
    }

    *index = new_index;

    for mut skybox in &mut skyboxes {
        let view_dimension = &CUBEMAPS[*index].2;
        skybox.image = asset_server.load_with_settings(
            CUBEMAPS[*index].0,
            |settings: &mut ImageLoaderSettings| {
                settings.view_dimension = view_dimension.clone();
            },
        );
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}
