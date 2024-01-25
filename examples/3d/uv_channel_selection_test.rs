//! This example demonstrates the use of the second UV channel for mapping various texture properties.
//! The spheres on the left use UV0 and those on the right use UV1 (which is just a scaled copy).
//! The spheres on the top test base colour and metal-roughness. Those on the bottom test normals and occlusion.

use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetPersistencePolicy,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .run();
}

/// Will rotate our shapes. Stolen from the shape example.
#[derive(Component)]
struct Shape;

const X_EXTENT: f32 = 3.0;
const SCALE_FACTOR: f32 = 2.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let image_colour = Some(images.add(uv_debug_texture()));
    let image_normal = Some(images.add(grid_normal_map_texture()));
    let image_metallic_roughness = Some(images.add(metallic_roughness_uv_texture()));
    let image_occlusion = Some(images.add(grid_occlusion_map_texture()));

    // Material using UV0 (default) for base colour, metallic/roughness.
    let debug_material_1 = materials.add(StandardMaterial {
        base_color_texture: image_colour.clone(),
        metallic: 1.0,
        perceptual_roughness: 1.0,
        metallic_roughness_texture: image_metallic_roughness.clone(),
        normal_map_texture: image_normal.clone(),
        ..default()
    });

    // Material using UV1 for base colour, metallic/roughness.
    let debug_material_2 = materials.add(StandardMaterial {
        base_color_texture: image_colour.clone(),
        base_color_texture_uv_channel: 1,
        metallic: 1.0,
        perceptual_roughness: 1.0,
        metallic_roughness_texture: image_metallic_roughness.clone(),
        metallic_roughness_texture_uv_channel: 1,
        ..default()
    });

    // Normal and occlusion, UV0 (default).
    let debug_material_3 = materials.add(StandardMaterial {
        normal_map_texture: image_normal.clone(),
        occlusion_texture: image_occlusion.clone(),
        ..default()
    });

    // Normal and occlusion, UV1.
    let debug_material_4 = materials.add(StandardMaterial {
        normal_map_texture: image_normal.clone(),
        normal_map_texture_uv_channel: 1,
        occlusion_texture: image_occlusion.clone(),
        occlusion_texture_uv_channel: 1,
        ..default()
    });

    let mut new_mesh = Mesh::from(shape::UVSphere::default());
    new_mesh
        .generate_tangents()
        .expect("Failed to generate tangents. Oh my.");

    // Attempt to copy UVs from the first channel.
    if let Some(uv0) = new_mesh
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .and_then(|attr| attr.as_float2())
    {
        let uv1: Vec<[f32; 2]> = uv0
            .iter()
            .map(|&[u, v]| [u * SCALE_FACTOR, v * SCALE_FACTOR])
            .collect();
        new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv1);
    } else {
        eprintln!("Failed to copy UVs!");
    }

    let new_mesh_handle = meshes.add(new_mesh);

    // Upper-left sphere (material 1).
    commands.spawn((
        PbrBundle {
            mesh: new_mesh_handle.clone(),
            material: debug_material_1.clone(),
            transform: Transform::from_xyz(-X_EXTENT / 2.0, 2.0, 0.0)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        Shape,
    ));

    // Upper-right sphere (material 2).
    commands.spawn((
        PbrBundle {
            mesh: new_mesh_handle.clone(),
            material: debug_material_2.clone(),
            transform: Transform::from_xyz(X_EXTENT / 2.0, 2.0, 0.0)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        Shape,
    ));

    // Lower-left sphere (material 3).
    commands.spawn((
        PbrBundle {
            mesh: new_mesh_handle.clone(),
            material: debug_material_3.clone(),
            transform: Transform::from_xyz(-X_EXTENT / 2.0, -2.0, 0.0)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        Shape,
    ));

    // Lower-right sphere (material 3).
    commands.spawn((
        PbrBundle {
            mesh: new_mesh_handle.clone(),
            material: debug_material_4.clone(),
            transform: Transform::from_xyz(X_EXTENT / 2.0, -2.0, 0.0)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        Shape,
    ));

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500000.0,
            range: 100.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 6., 12.0)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 150.0,
        },
    ));
}

fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

/// Creates a colourful test pattern. Stolen from 3D shapes example.
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

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetPersistencePolicy::Unload,
    )
}

/// Creates a pattern for testing metallic-roughness. It alternatives shiny metal and rough non-metal.
fn metallic_roughness_uv_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    // Define the color values for metallic and roughness
    // Metallic (B): 1.0 = 255, 0.0 = 0
    // Roughness (G): 1.0 = 255, 0.0 = 0
    const METALLIC_FULL_ROUGHNESS_ZERO: [u8; 4] = [0, 0, 255, 255]; // Blue channel full, Green channel zero
    const METALLIC_ZERO_ROUGHNESS_FULL: [u8; 4] = [0, 255, 0, 255]; // Blue channel zero, Green channel full

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let offset = (y * TEXTURE_SIZE + x) * 4;
            if (x + y) % 2 == 0 {
                texture_data[offset..offset + 4].copy_from_slice(&METALLIC_FULL_ROUGHNESS_ZERO);
            } else {
                texture_data[offset..offset + 4].copy_from_slice(&METALLIC_ZERO_ROUGHNESS_FULL);
            }
        }
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetPersistencePolicy::Unload,
    )
}

/// Creates a test normal map, which is just grooves along a grid pattern.
fn grid_normal_map_texture() -> Image {
    const TEXTURE_SIZE: usize = 512;
    const GRID_SIZE: usize = 8;
    const CREASE_WIDTH: usize = 2;
    const NORMAL_UP: [u8; 4] = [128, 128, 255, 255]; // Normal facing upwards
    const NORMAL_CREASE_UP: [u8; 4] = [192, 192, 255, 255]; // Slightly up for one side of the crease
    const NORMAL_CREASE_DOWN: [u8; 4] = [64, 64, 255, 255]; // Slightly down for the other side of the crease

    let mut texture_data = vec![0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    let grid_step = TEXTURE_SIZE / GRID_SIZE;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let offset = (y * TEXTURE_SIZE + x) * 4;
            let dx = x % grid_step;
            let dy = y % grid_step;
            let is_crease_x = dx < CREASE_WIDTH;
            let is_crease_y = dy < CREASE_WIDTH;
            let is_crease = is_crease_x || is_crease_y;

            let normal = if is_crease {
                if (is_crease_x && dx * 2 < CREASE_WIDTH) || (is_crease_y && dy * 2 < CREASE_WIDTH)
                {
                    &NORMAL_CREASE_UP
                } else {
                    &NORMAL_CREASE_DOWN
                }
            } else {
                &NORMAL_UP
            };

            texture_data[offset..offset + 4].copy_from_slice(normal);
        }
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8Unorm,
        RenderAssetPersistencePolicy::Unload,
    )
}

/// Creates an occlusion test pattern, which is just a grid of black lines aligning with the normal pattern above.
fn grid_occlusion_map_texture() -> Image {
    const TEXTURE_SIZE: usize = 512;
    const GRID_SIZE: usize = 8;
    const CREASE_WIDTH: usize = 1;
    const WHITE: [u8; 4] = [255, 255, 255, 255]; // White color
    const BLACK: [u8; 4] = [0, 0, 0, 255]; // Black color for the grid lines

    let mut texture_data = vec![0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    let grid_step = TEXTURE_SIZE / GRID_SIZE;

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let offset = (y * TEXTURE_SIZE + x) * 4;
            let dx = x % grid_step;
            let dy = y % grid_step;
            let is_crease = dx < CREASE_WIDTH || dy < CREASE_WIDTH;

            texture_data[offset..offset + 4].copy_from_slice(if is_crease {
                &BLACK
            } else {
                &WHITE
            });
        }
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8Unorm, // Use linear color space
        RenderAssetPersistencePolicy::Unload,
    )
}
