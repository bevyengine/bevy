use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;

// Notice "ImagePlugin::default_nearest()" in main, because our texture is very low res
// we want it to look pixelated, instead of the default ImagePlugin::default_linear() which will
// smooth our pixelated texture. However, default_linear is usually better for high res textures.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .run();
}

// Rotate the cube to show off all the sides.
fn rotate(mut query: Query<&mut Transform, With<Handle<Mesh>>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_z(time.delta_seconds() / 1.2);
        transform.rotate_x(time.delta_seconds() / 2.0);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Importing the custom texture.
    let custom_texture_handle: Handle<Image> =
        asset_server.load("textures/custom_image_for_example.png");
    // Creating and saving a handle to the mesh.
    let cube_mesh_handle: Handle<Mesh> = meshes.add(create_cube_mesh());

    // Rendering the mesh with the custom texture using a PbrBundle.
    commands.spawn(PbrBundle {
        mesh: cube_mesh_handle,
        material: materials.add(StandardMaterial {
            base_color_texture: Some(custom_texture_handle),
            ..default()
        }),
        ..default()
    });

    // Transform for the camera and lighting, looking at (0,0,0) (the position of the mesh).
    let camera_and_light_transform =
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y);

    // Camera in 3D space.
    commands.spawn(Camera3dBundle {
        transform: camera_and_light_transform.clone(),
        ..default()
    });

    // Lighting up the scene.
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 100.0,
            ..default()
        },
        transform: camera_and_light_transform,
        ..default()
    });
}

fn create_cube_mesh() -> Mesh {
    let mut cube_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    #[rustfmt::skip]
    cube_mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            // top (facing towards +y)
            [0.0, 1.0, 0.0], // vertex with index 0
            [1.0, 1.0, 0.0], // vertex wtih index 1
            [1.0, 1.0, 1.0], // etc. until 23
            [0.0, 1.0, 1.0],
            // bottom  `  `  (-y)
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            // forward `  `  (+x)
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            // back    `  `  (-x)
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
            // right   `  `  (+z)
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
            // left    `  `  (-z)
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
    );

    // Take a look at the custom image (assets/textures/custom_image_for_example.png)
    // so the UV coords will make more sense (note (0.0, 0.0) = Top-Left in UV mapping).
    #[rustfmt::skip]
    cube_mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            // Assigning the UV coords for the top side.
            [0.5, 0.0], [0.0, 0.0], [0.0, 0.5], [0.5, 0.5],
            // Assigning the UV coords for the bottom side.
            [0.5, 0.5], [0.5, 0.0], [1.0, 0.0], [1.0, 0.5],
            // Assigning the UV coords for the forward side.
            [0.5, 1.0], [0.5, 0.5], [1.0, 0.5], [1.0, 1.0],
            // Assigning the UV coords for the back side (same as forward because they have the
            // same texture)
            [0.5, 1.0], [0.5, 0.5], [1.0, 0.5], [1.0, 1.0],
            // Assigning the UV coords for the right side.
            [0.0, 1.0], [0.0, 0.5], [0.5, 0.5], [0.5, 1.0],
            // Assigning the UV coords for the left side (same as right).
            [0.0, 1.0], [0.0, 0.5], [0.5, 0.5], [0.5, 1.0],
        ],
    );

    // When it comes to smooth, and simple meshes, normals are as simple as the direction of the flat surface
    // Assign normals to allow for correct lighting calculations.
    #[rustfmt::skip]
    cube_mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![
            // Normals for the top side (towards +y)
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            // Normals for the bottom side (towards -y)
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            // Normals for the forward side (towards +x)
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            // Normals for the back side (towards -x)
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            // Normals for the right side (towards +z)
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            // Normals for the left side (towards -z)
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
        ],
    );

    // Create the triangles out of the 24 vertices we created.
    // To construct a square, we need 2 triangles, therfore 12 traingles in total.
    // To construct a triangle, we need the indices of 3 of its defined vertices, adding them one
    // by one, in a counter-clockwise order (relative to the position of the viewer, the order
    // should appear counter-clockwise from the front of the triangle). Read more about how to correctly build a mesh manually
    // in the Bevy documentation, further examples and the implementation of the built-in shapes.
    #[rustfmt::skip]
    cube_mesh.set_indices(Some(Indices::U32(vec![
        0,3,1 , 1,3,2, // top (+y)
        4,5,7 , 5,6,7, // bottom (-y)
        8,11,9 , 9,11,10, // forward (+x)
        12,13,15 , 13,14,15, // backward (-x)
        16,19,17 , 17,19,18, // rightward (+z)
        20,21,23 , 21,22,23, // leftward (-z)
    ])));

    cube_mesh
}
