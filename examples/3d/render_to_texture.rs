//! Shows how to render to a texture. Useful for mirrors, UI, or exporting images.

use std::f32::consts::PI;

use bevy::{camera::visibility::RenderLayers, prelude::*, render::render_resource::TextureFormat};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (cube_rotator_system, rotator_system))
        .run();
}

// Marks the first pass cube (rendered to a texture.)
#[derive(Component)]
struct FirstPassCube;

// Marks the main pass cube, to which the texture is applied.
#[derive(Component)]
struct MainPassCube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // This is the texture that will be rendered to.
    let image = Image::new_target_texture(512, 512, TextureFormat::bevy_default());

    let image_handle = images.add(image);

    let cube_handle = meshes.add(Cuboid::new(4.0, 4.0, 4.0));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let first_pass_layer = RenderLayers::layer(1);

    // The cube that will be rendered to the texture.
    commands.spawn((
        Mesh3d(cube_handle),
        MeshMaterial3d(cube_material_handle),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        FirstPassCube,
        first_pass_layer.clone(),
    ));

    // Light
    // NOTE: we add the light to both layers so it affects both the rendered-to-texture cube, and the cube on which we display the texture
    // Setting the layer to RenderLayers::layer(0) would cause the main view to be lit, but the rendered-to-texture cube to be unlit.
    // Setting the layer to RenderLayers::layer(1) would cause the rendered-to-texture cube to be lit, but the main view to be unlit.
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        RenderLayers::layer(0).with(1),
    ));

    commands.spawn((
        Camera3d::default(),
        Camera {
            // render before the "main pass" camera
            order: -1,
            target: image_handle.clone().into(),
            clear_color: Color::WHITE.into(),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 15.0)).looking_at(Vec3::ZERO, Vec3::Y),
        first_pass_layer,
    ));

    let cube_size = 4.0;
    let cube_handle = meshes.add(Cuboid::new(cube_size, cube_size, cube_size));

    // This material has the texture that has been rendered.
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // Main pass cube, with material containing the rendered first pass texture.
    commands.spawn((
        Mesh3d(cube_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, 1.5).with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        MainPassCube,
    ));

    // The main pass camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Rotates the inner cube (first pass)
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<FirstPassCube>>) {
    for mut transform in &mut query {
        transform.rotate_x(1.5 * time.delta_secs());
        transform.rotate_z(1.3 * time.delta_secs());
    }
}

/// Rotates the outer cube (main pass)
fn cube_rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<MainPassCube>>) {
    for mut transform in &mut query {
        transform.rotate_x(1.0 * time.delta_secs());
        transform.rotate_y(0.7 * time.delta_secs());
    }
}
