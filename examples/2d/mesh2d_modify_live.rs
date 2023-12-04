//! Shows how to change the attributes of a polygonal [`Mesh`], generated from a [`Quad`] primitive, in a 2D scene.

use bevy::{
    prelude::*,
    sprite::MaterialMesh2dBundle,
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
    mesh_handle: Handle<Mesh>,
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
        Color::BLUE.as_rgba_f32(),
        Color::RED.as_rgba_f32(),
        Color::BLUE.as_rgba_f32(),
    ];
    println!("vertex_colors: {:?}", vertex_colors);

    // Insert the vertex colors as an attribute
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors.clone());

    // Add the mesh to the asset storage and get a handle to it that will allow us to access it later
    let mesh_handle: Handle<Mesh> = meshes.add(mesh).into();

    // Track the colors using the DynamicMesh component
    commands.spawn(DynamicMesh {
        mesh_handle: mesh_handle.clone(),
        vertex_colors: vertex_colors.clone(),
    });

    // Spawn camera
    commands.spawn(Camera2dBundle::default());

    // Spawn the quad with vertex colors
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone().into(),
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
    for data in &query {
        let handle = data.mesh_handle.clone();
        let mesh = meshes.get_mut(&handle).unwrap();
        // let mesh = meshes.get(&handle).unwrap();
        let mut vertex_colors = data.vertex_colors.clone();
        let t = time.elapsed_seconds() as f32;
        vertex_colors = _equalize_colors(vertex_colors, t);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
    }
}

const DIFF_CONST: f32 = 0.2;

// Implement the diffusion logic
// - for all colors, A is one of those colors, B is average of all colors
// - at time t, the diffused color A = (A * exp(-t) + B * (1 - exp(-t)))
fn _equalize_colors(vertex_colors: Vec<[f32; 4]>, t: f32) -> Vec<[f32; 4]> {
    let mut new_colors = vertex_colors.clone();
    let mut sum: [f32; 4] = [0., 0., 0., 0.];
    for color in &vertex_colors {
        sum[0] += color[0];
        sum[1] += color[1];
        sum[2] += color[2];
        sum[3] += color[3];
    }
    let avg: [f32; 4] = [
        sum[0] / vertex_colors.len() as f32,
        sum[1] / vertex_colors.len() as f32,
        sum[2] / vertex_colors.len() as f32,
        sum[3] / vertex_colors.len() as f32,
    ];
    let tt = t * DIFF_CONST;
    for color in &mut new_colors {
        color[0] = color[0] * (-tt).exp() + avg[0] * (1. - (-tt).exp());
        color[1] = color[1] * (-tt).exp() + avg[1] * (1. - (-tt).exp());
        color[2] = color[2] * (-tt).exp() + avg[2] * (1. - (-tt).exp());
        color[3] = color[3] * (-tt).exp() + avg[3] * (1. - (-tt).exp());
    }
    new_colors
}