use bevy::{
    math::Vec3,
    pbr2::{PbrBundle, PointLightBundle, StandardMaterial},
    prelude::{App, Assets, Commands, ResMut, Transform},
    render2::{
        camera::PerspectiveCameraBundle,
        mesh::{shape, Mesh, VertexAttributeValues},
    },
    PipelinedDefaultPlugins,
};

/// This example illustrates how to use the vertex colors attribute.
fn main() {
    App::new()
        .add_plugins(PipelinedDefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // create a generic cube
    let mut cube_with_colors = Mesh::from(shape::Cube { size: 2.0 });

    // set some nice nice colors!
    cube_with_colors.set_attribute(
        Mesh::ATTRIBUTE_COLOR,
        // NOTE: the attribute count has to be consistent across all attributes, otherwise bevy
        // will panic.
        VertexAttributeValues::from(vec![
            // top
            [0.79, 0.73, 0.07, 1.],
            [0.74, 0.14, 0.29, 1.],
            [0.08, 0.55, 0.74, 1.],
            [0.20, 0.27, 0.29, 1.],
            // bottom
            [0.79, 0.73, 0.07, 1.],
            [0.74, 0.14, 0.29, 1.],
            [0.08, 0.55, 0.74, 1.],
            [0.20, 0.27, 0.29, 1.],
            // right
            [0.79, 0.73, 0.07, 1.],
            [0.74, 0.14, 0.29, 1.],
            [0.08, 0.55, 0.74, 1.],
            [0.20, 0.27, 0.29, 1.],
            // left
            [0.79, 0.73, 0.07, 1.],
            [0.74, 0.14, 0.29, 1.],
            [0.08, 0.55, 0.74, 1.],
            [0.20, 0.27, 0.29, 1.],
            // front
            [0.79, 0.73, 0.07, 1.],
            [0.74, 0.14, 0.29, 1.],
            [0.08, 0.55, 0.74, 1.],
            [0.20, 0.27, 0.29, 1.],
            // back
            [0.79, 0.73, 0.07, 1.],
            [0.74, 0.14, 0.29, 1.],
            [0.08, 0.55, 0.74, 1.],
            [0.20, 0.27, 0.29, 1.],
        ]),
    );
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(cube_with_colors), // use our cube with vertex colors
        material: materials.add(Default::default()),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(3.0, 5.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
