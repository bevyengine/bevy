//! Shows how to change the attributes of a polygonal [`Mesh`], generated from a [`Quad`] primitive, in a 2D scene.

use std::collections::HashMap;
use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, equalize_colors)
        .run();
}

#[derive(Component)]
struct DynamicMesh {
    mesh_handle: Mesh2dHandle,
    vertex_colors: Vec<[f32; 4]>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Build a default quad mesh
    let mut mesh = Mesh::from(shape::Quad::default());
    // Build vertex colors for the quad. One entry per vertex (the corners of the quad)
    let vertex_colors: Vec<[f32; 4]> = vec![
        Color::RED.as_rgba_f32(),
        Color::GREEN.as_rgba_f32(),
        Color::BLUE.as_rgba_f32(),
        Color::WHITE.as_rgba_f32(),
    ];
    println!("vertex_colors: {:?}", vertex_colors);

    // Insert the vertex colors as an attribute
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors.clone());

    // Add the mesh to the asset storage and get a handle to it that will allow us to access it later
    let mesh_handle: Mesh2dHandle = meshes.add(mesh).into();

    // Track the colors using the DynamicMesh component
    commands.spawn(DynamicMesh {
        mesh_handle: mesh_handle.clone(),
        vertex_colors: vertex_colors.clone(),
    });

    // Spawn camera
    commands.spawn(Camera2dBundle::default());

    // Spawn the quad with vertex colors
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone(),
        transform: Transform::from_translation(Vec3::new(0., 0., 0.))
            .with_scale(Vec3::splat(512.)),
        material: materials.add(ColorMaterial::default()),
        ..default()
    });
}

// Change the colors of the quad proportionally to the time elapsed
fn equalize_colors(
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<&DynamicMesh>,
) {
    println!("equalize_colors: time={}", time.elapsed_seconds());
}