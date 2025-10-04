//! Shows how to modify mesh assets after spawning.

use bevy::{
    asset::RenderAssetUsages, gltf::GltfLoaderSettings,
    input::common_conditions::input_just_pressed, mesh::VertexAttributeValues, prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, spawn_text))
        .add_systems(
            Update,
            alter_handle.run_if(input_just_pressed(KeyCode::Space)),
        )
        .add_systems(
            Update,
            alter_mesh.run_if(input_just_pressed(KeyCode::Enter)),
        )
        .run();
}

#[derive(Component, Debug)]
enum Shape {
    Cube,
    Sphere,
}

impl Shape {
    fn get_model_path(&self) -> String {
        match self {
            Shape::Cube => "models/cube/cube.gltf".into(),
            Shape::Sphere => "models/sphere/sphere.gltf".into(),
        }
    }

    fn set_next_variant(&mut self) {
        *self = match self {
            Shape::Cube => Shape::Sphere,
            Shape::Sphere => Shape::Cube,
        }
    }
}

#[derive(Component, Debug)]
struct Left;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let left_shape = Shape::Cube;
    let right_shape = Shape::Cube;

    // In normal use, you can call `asset_server.load`, however see below for an explanation of
    // `RenderAssetUsages`.
    let left_shape_model = asset_server.load_with_settings(
        GltfAssetLabel::Primitive {
            mesh: 0,
            // This field stores an index to this primitive in its parent mesh. In this case, we
            // want the first one. You might also have seen the syntax:
            //
            //     models/cube/cube.gltf#Scene0
            //
            // which accomplishes the same thing.
            primitive: 0,
        }
        .from_asset(left_shape.get_model_path()),
        // `RenderAssetUsages::all()` is already the default, so the line below could be omitted.
        // It's helpful to know it exists, however.
        //
        // `RenderAssetUsages` tell Bevy whether to keep the data around:
        //   - for the GPU (`RenderAssetUsages::RENDER_WORLD`),
        //   - for the CPU (`RenderAssetUsages::MAIN_WORLD`),
        //   - or both.
        // `RENDER_WORLD` is necessary to render the mesh, `MAIN_WORLD` is necessary to inspect
        // and modify the mesh (via `ResMut<Assets<Mesh>>`).
        //
        // Since most games will not need to modify meshes at runtime, many developers opt to pass
        // only `RENDER_WORLD`. This is more memory efficient, as we don't need to keep the mesh in
        // RAM. For this example however, this would not work, as we need to inspect and modify the
        // mesh at runtime.
        |settings: &mut GltfLoaderSettings| settings.load_meshes = RenderAssetUsages::all(),
    );

    // Here, we rely on the default loader settings to achieve a similar result to the above.
    let right_shape_model = asset_server.load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset(right_shape.get_model_path()),
    );

    // Add a material asset directly to the materials storage
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.8, 0.6),
        ..default()
    });

    commands.spawn((
        Left,
        Name::new("Left Shape"),
        Mesh3d(left_shape_model),
        MeshMaterial3d(material_handle.clone()),
        Transform::from_xyz(-3.0, 0.0, 0.0),
        left_shape,
    ));

    commands.spawn((
        Name::new("Right Shape"),
        Mesh3d(right_shape_model),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(3.0, 0.0, 0.0),
        right_shape,
    ));

    commands.spawn((
        Name::new("Point Light"),
        PointLight::default(),
        Transform::from_xyz(4.0, 5.0, 4.0),
    ));

    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_text(mut commands: Commands) {
    commands.spawn((
        Name::new("Instructions"),
        Text::new(
            "Space: swap meshes by mutating a Handle<Mesh>\n\
            Return: mutate the mesh itself, changing all copies of it",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn alter_handle(
    asset_server: Res<AssetServer>,
    right_shape: Single<(&mut Mesh3d, &mut Shape), Without<Left>>,
) {
    // Mesh handles, like other parts of the ECS, can be queried as mutable and modified at
    // runtime. We only spawned one shape without the `Left` marker component.
    let (mut mesh, mut shape) = right_shape.into_inner();

    // Switch to a new Shape variant
    shape.set_next_variant();

    // Modify the handle associated with the Shape on the right side. Note that we will only
    // have to load the same path from storage media once: repeated attempts will re-use the
    // asset.
    mesh.0 = asset_server.load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset(shape.get_model_path()),
    );
}

fn alter_mesh(
    mut is_mesh_scaled: Local<bool>,
    left_shape: Single<&Mesh3d, With<Left>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Obtain a mutable reference to the Mesh asset.
    let Some(mesh) = meshes.get_mut(*left_shape) else {
        return;
    };

    // Now we can directly manipulate vertices on the mesh. Here, we're just scaling in and out
    // for demonstration purposes. This will affect all entities currently using the asset.
    //
    // To do this, we need to grab the stored attributes of each vertex. `Float32x3` just describes
    // the format in which the attributes will be read: each position consists of an array of three
    // f32 corresponding to x, y, and z.
    //
    // `ATTRIBUTE_POSITION` is a constant indicating that we want to know where the vertex is
    // located in space (as opposed to which way its normal is facing, vertex color, or other
    // details).
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        // Check a Local value (which only this system can make use of) to determine if we're
        // currently scaled up or not.
        let scale_factor = if *is_mesh_scaled { 0.5 } else { 2.0 };

        for position in positions.iter_mut() {
            // Apply the scale factor to each of x, y, and z.
            position[0] *= scale_factor;
            position[1] *= scale_factor;
            position[2] *= scale_factor;
        }

        // Flip the local value to reverse the behavior next time the key is pressed.
        *is_mesh_scaled = !*is_mesh_scaled;
    }
}
