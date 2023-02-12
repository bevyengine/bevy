//! This examples compares MSAA (Multi-Sample Anti-Aliasing) and FXAA (Fast Approximate Anti-Aliasing).

use std::f32::consts::PI;

use bevy::{
    math::vec2,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    render::{
        render_resource::{Extent3d, SamplerDescriptor, TextureDimension, TextureFormat},
        texture::ImageSampler,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let cam_trans =
        Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y);
    // plane
    //commands.spawn(PbrBundle {
    //    mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
    //    material: materials.add(StandardMaterial {
    //        base_color: Color::rgb(0.3, 0.5, 0.3).into(),
    //        perceptual_roughness: 0.5,
    //        ..default()
    //    }),
    //    ..default()
    //});

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Quad {
            size: vec2(16.0, 9.0) * 0.1,
            flip: false,
        })),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("../../dragonscene_ap0_v01_1001.exr")),
            unlit: true,
            ..default()
        }),
        transform: Transform::from_xyz(0.2, 0.38, 0.2).with_rotation(cam_trans.rotation),
        ..default()
    });

    let cube_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // cubes
    for i in 0..5 {
        commands.spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.25 })),
            material: cube_material.clone(),
            transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
            ..default()
        });
    }

    // Flight Helmet
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
        ..default()
    });

    // light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            //illuminance: 30000.0,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            PI * -0.15,
            PI * -0.15,
        )),
        cascade_shadow_config: CascadeShadowConfigBuilder {
            maximum_distance: 3.0,
            first_cascade_far_bound: 0.9,
            ..default()
        }
        .into(),
        ..default()
    });

    // camera
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: false, // Works with and without hdr
                ..default()
            },
            transform: cam_trans,
            ..default()
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        });
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    let mut img = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    img.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor::default());
    img
}
