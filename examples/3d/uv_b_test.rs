//! This example demonstrates the built-in 3d shapes in Bevy.
//! The scene includes a patterned texture and a rotation for visualizing the normals and UVs.

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

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const X_EXTENT: f32 = 7.0;
const SCALE_FACTOR: f32 = 2.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material_1 = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        base_color_texture_uv_channel: 0,
        ..default()
    });
    let debug_material_2 = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        base_color_texture_uv_channel: 1,
        ..default()
    });

    let mut new_mesh = Mesh::from(shape::UVSphere::default());

    // Attempt to copy UVs from the first channel
    if let Some(uv0) = new_mesh.attribute(Mesh::ATTRIBUTE_UV_0).and_then(|attr| attr.as_float2()) {
        let uv1: Vec<[f32; 2]> = uv0.iter().map(|&[u, v]| [u * SCALE_FACTOR, v * SCALE_FACTOR]).collect();
        new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv1);
    } else {
        eprintln!("Failed to copy UVs!");
        eprintln!("Available attributes:");
        for (attribute_id, _) in new_mesh.attributes() {
            eprintln!(" - {:?}", attribute_id);
        }
    }

    let new_mesh_handle = meshes.add(new_mesh);

    commands.spawn((
        PbrBundle {
            mesh: new_mesh_handle.clone(),
            material: debug_material_1.clone(),
            transform: Transform::from_xyz(
                -X_EXTENT / 2.0,
                2.0,
                0.0,
            )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        Shape,
    ));

    commands.spawn((
        PbrBundle {
            mesh: new_mesh_handle,
            material: debug_material_2.clone(),
            transform: Transform::from_xyz(
                X_EXTENT / 4.0,
                2.0,
                0.0,
            )
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

    // ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(50.0)),
        material: materials.add(Color::SILVER),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });
}

fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
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
