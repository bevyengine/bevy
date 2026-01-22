//! Demonstrates connecting color target input and output of multiple cameras.
//!

use bevy::{
    camera::color_target::{
        MainColorTarget, MainColorTargetReadsFrom, NoAutoConfiguredMainColorTarget,
        WithMainColorTarget, MAIN_COLOR_TARGET_DEFAULT_USAGES,
    },
    prelude::*,
};
use bevy_image::ToExtents;
use bevy_render::render_resource::{
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let transform = Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y);
    let uv_checker_image = asset_server.load::<Image>("textures/uv_checker_bw.png");

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    let main_color_target = commands
        .spawn(MainColorTarget::new(
            images.add(Image {
                data: None,
                texture_descriptor: TextureDescriptor {
                    label: Some("main_texture_a"),
                    size: UVec2::new(1024, 1024).to_extents(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: MAIN_COLOR_TARGET_DEFAULT_USAGES,
                    view_formats: &[],
                },
                ..Default::default()
            }),
            Some(images.add(Image {
                data: None,
                texture_descriptor: TextureDescriptor {
                    label: Some("main_texture_b"),
                    size: UVec2::new(1024, 1024).to_extents(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: MAIN_COLOR_TARGET_DEFAULT_USAGES,
                    view_formats: &[],
                },
                ..Default::default()
            })),
            Some(images.add(Image {
                data: None,
                texture_descriptor: TextureDescriptor {
                    label: Some("main_texture_multisampled"),
                    size: UVec2::new(1024, 1024).to_extents(),
                    mip_level_count: 1,
                    sample_count: 4, // MSAAx4
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
                ..Default::default()
            })),
        ))
        .id();

    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::None,
            ..Default::default()
        },
        NoAutoConfiguredMainColorTarget,
        MainColorTargetReadsFrom(uv_checker_image),
        WithMainColorTarget(main_color_target),
        transform,
    ));
}
