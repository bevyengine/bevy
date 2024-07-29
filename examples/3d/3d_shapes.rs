//! This example demonstrates the built-in 3d shapes in Bevy.
//! The scene includes a patterned texture and a rotation for visualizing the normals and UVs.
//!
//! You can toggle wireframes with the space bar except on wasm. Wasm does not support
//! `POLYGON_MODE_LINE` on the gpu.

use std::f32::consts::PI;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::{
    color::palettes::basic::SILVER,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            #[cfg(not(target_arch = "wasm32"))]
            WireframePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                rotate,
                #[cfg(not(target_arch = "wasm32"))]
                toggle_wireframe,
            ),
        )
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(ConicalFrustum::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
    ];

    let extrusions = [
        meshes.add(Extrusion::new(Rectangle::default(), 1.)),
        meshes.add(Extrusion::new(Capsule2d::default(), 1.)),
        meshes.add(Extrusion::new(Annulus::default(), 1.)),
        meshes.add(Extrusion::new(Circle::default(), 1.)),
        meshes.add(Extrusion::new(Ellipse::default(), 1.)),
        meshes.add(Extrusion::new(RegularPolygon::default(), 1.)),
        meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                    2.0,
                    Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    let num_extrusions = extrusions.len();

    for (i, shape) in extrusions.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    -EXTRUSION_X_EXTENT / 2.
                        + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                    2.0,
                    -Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    // ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10)),
        material: materials.add(Color::from(SILVER)),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });

    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn(
        TextBundle::from_section("Press space to toggle wireframes", TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            }),
    );
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
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}
