use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::pipeline::PrimitiveTopology;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let origin = Vec3::new(0.0, 0.0, 0.0);
    let right = Vec3::new(1.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    #[rustfmt::skip]
    let positions = vec![
        origin + up, 
        origin + up + right, 
        origin, 
        origin + right
        ];

    let origin = Vec2::new(0.0, 0.0);
    let right = Vec2::new(1.0, 0.0);
    let up = Vec2::new(0.0, 1.0);

    #[rustfmt::skip]
    let uvs = vec![
        origin + up, 
        origin + up + right, 
        origin, 
        origin + right
        ];

    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let indices = Indices::U32(vec![0, 1, 2, 3, 2, 1]);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(mesh),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });

    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 5.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
