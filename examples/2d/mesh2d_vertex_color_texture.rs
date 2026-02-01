//! Shows how to render a polygonal [`Mesh`], generated from a [`Rectangle`] primitive, in a 2D scene.
//! Adds a texture and colored vertices, giving per-vertex tinting.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
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
    let mut mesh = Mesh::from(Rectangle::default());
    // Build vertex colors for the quad. One entry per vertex (the corners of the quad)
    let vertex_colors: Vec<[f32; 4]> = vec![
        LinearRgba::RED.to_f32_array(),
        LinearRgba::GREEN.to_f32_array(),
        LinearRgba::BLUE.to_f32_array(),
        LinearRgba::WHITE.to_f32_array(),
    ];
    // Insert the vertex colors as an attribute
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);

    let mesh_handle = meshes.add(mesh);

    commands.spawn(Camera2d);

    // Spawn the quad with vertex colors
    commands.spawn((
        Mesh2d(mesh_handle.clone()),
        MeshMaterial2d(materials.add(ColorMaterial::default())),
        Transform::from_translation(Vec3::new(-96., 0., 0.)).with_scale(Vec3::splat(128.)),
    ));

    // Spawning the quad with vertex colors and a texture results in tinting
    commands.spawn((
        Mesh2d(mesh_handle),
        MeshMaterial2d(materials.add(texture_handle)),
        Transform::from_translation(Vec3::new(96., 0., 0.)).with_scale(Vec3::splat(128.)),
    ));
}
