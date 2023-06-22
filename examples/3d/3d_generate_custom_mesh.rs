// ! This example demonstrates how to create a custom mesh, and assign a custom UV mapping for a
// ! cusotm texture. And how to change the UV mapping at run-time.
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::PrimitiveTopology;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_y)
        // .add_systems(Update, rotate_x)
        // .add_systems(Update, rotate_z)
        .add_systems(Update, input_handler)
        .run();
}

// Rotate the cube to show off all the sides.
#[allow(dead_code)]
fn rotate_y(mut query: Query<&mut Transform, With<Handle<Mesh>>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 1.2);
    }
}

// Optional system to rotate around the x axis.
#[allow(dead_code)]
fn rotate_x(mut query: Query<&mut Transform, With<Handle<Mesh>>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_x(time.delta_seconds() / 1.2);
    }
}

// Optional system to rotate around the z axis.
#[allow(dead_code)]
fn rotate_z(mut query: Query<&mut Transform, With<Handle<Mesh>>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_z(time.delta_seconds() / 1.2);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Importing the custom texture.
    let custom_texture_handle: Handle<Image> = asset_server.load("textures/array_texture.png");
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
        transform: camera_and_light_transform,
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

// System to recieve input from the user, check out examples/input/ for more examples about user
// input.
fn input_handler(
    keyboard_input: Res<Input<KeyCode>>,
    mesh_query: Query<&Handle<Mesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        let mesh_handle = mesh_query.get_single().expect("Query not successful");
        let mesh = meshes.get_mut(&mesh_handle).unwrap();
        toggle_texture(mesh);
    }
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
            // bottom `  `  (-y)
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            // right  `  `  (+x)
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            // left   `  `  (-x)
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
            // back   `  `  (+z)
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
            // forward ` `  (-z)
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
            [0.0, 0.2], [0.0, 0.0], [1.0, 0.0], [1.0, 0.25],
            // Assigning the UV coords for the bottom side.
            [0.0, 0.45], [0.0, 0.25], [1.0, 0.25], [1.0, 0.45],
            // Assigning the UV coords for the right side.
            [1.0, 0.45], [0.0, 0.45], [0.0, 0.2], [1.0, 0.2],
            // Assigning the UV coords for the left side. 
            [1.0, 0.45], [0.0, 0.45], [0.0, 0.2], [1.0, 0.2],
            // Assigning the UV coords for the back side.
            [0.0, 0.45], [0.0, 0.2], [1.0, 0.2], [1.0, 0.45],
            // Assigning the UV coords for the forward side.
            [0.0, 0.45], [0.0, 0.2], [1.0, 0.2], [1.0, 0.45],
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
            // Normals for the right side (towards +x)
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            // Normals for the left side (towards -x)
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            // Normals for the back side (towards +z)
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            // Normals for the forward side (towards -z)
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
        ],
    );

    // Create the triangles out of the 24 vertices we created.
    // To construct a square, we need 2 triangles, therfore 12 traingles in total.
    // To construct a triangle, we need the indices of its 3 defined vertices, adding them one
    // by one, in a counter-clockwise order (relative to the position of the viewer, the order
    // should appear counter-clockwise from the front of the triangle). Read more about how to correctly build a mesh manually
    // in the Bevy documentation of a Mesh, further examples and the implementation of the built-in shapes.
    #[rustfmt::skip]
    cube_mesh.set_indices(Some(Indices::U32(vec![
        0,3,1 , 1,3,2, // triangles making up the top (+y) facing side.
        4,5,7 , 5,6,7, // bottom (-y) 
        8,11,9 , 9,11,10, // right (+x)
        12,13,15 , 13,14,15, // left (-x)
        16,19,17 , 17,19,18, // back (+z)
        20,21,23 , 21,22,23, // forward (-z)
    ])));

    cube_mesh
}

// Function that changes the UV mapping of the mesh, to apply the other texture.
fn toggle_texture(mesh_to_change: &mut Mesh) {
    // Use .attributes_mut to iterate over all of the attributes of the mesh, and get a mutable
    // reference to them.
    for attribute in mesh_to_change.attributes_mut() {
        // We only want to change the UV attributes.
        if attribute.0 == Mesh::ATTRIBUTE_UV_0.id {
            // VertexAttributeValues could have multiple different fromats, because we are dealing
            // with a UV attribute, we expect the format to be Float32x2 (same format we used to
            // define in lines 136 - 152).
            match attribute.1 {
                VertexAttributeValues::Float32x2(ref mut coordinates) => {
                    // Iterate all over the UV coordinates, changing them to the coordinate
                    // corresponding to the other texture.
                    for uv_coord in coordinates.iter_mut() {
                        if (uv_coord[1] + 0.5) < 1.0 {
                            uv_coord[1] += 0.5;
                        } else {
                            uv_coord[1] -= 0.5;
                        }
                    }
                }
                _ => panic!("expected Float32x2 for UV attributes"),
            }
        }
    }
}
