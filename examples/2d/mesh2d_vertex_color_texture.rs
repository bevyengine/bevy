//! Shows how to render a polygonal [`Mesh`], generated from a [`Quad`] primitive, in a 2D scene.
//! Adds a texture and colored vertices, giving per-vertex tinting.

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Load the Bevy logo as a texture
    let texture_handle = asset_server.load("branding/banner.png");
    // Build a default quad mesh
    let mut mesh = Mesh::from(shape::Quad::default());
    // Build vertex colors for the quad. One entry per vertex (the corners of the quad)
    let vertex_colors: Vec<[f32; 4]> = vec![
        Color::RED.as_rgba_f32(),
        Color::GREEN.as_rgba_f32(),
        Color::BLUE.as_rgba_f32(),
        Color::WHITE.as_rgba_f32(),
    ];
    // Insert the vertex colors as an attribute
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
    // Spawn
    commands.spawn_bundle(Camera2dBundle::default());
    commands.spawn_bundle(MaterialMesh2dBundle {
        mesh: meshes.add(mesh).into(),
        transform: Transform::default().with_scale(Vec3::splat(128.)),
        material: materials.add(ColorMaterial::from(texture_handle)),
        ..default()
    });
}
