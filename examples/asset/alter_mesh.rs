//! Shows how to modify mesh assets after spawning.

use bevy::prelude::*;
use bevy_render::mesh::VertexAttributeValues;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, instructions))
        .add_systems(Update, keyboard_controls)
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

    let left_shape_model = asset_server.load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset(left_shape.get_model_path()),
    );
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
        PbrBundle {
            mesh: left_shape_model,
            material: material_handle.clone(),
            transform: Transform::from_xyz(-3.0, 0.0, 0.0),
            ..default()
        },
        left_shape,
    ));

    commands.spawn((
        Name::new("Right Shape"),
        PbrBundle {
            mesh: right_shape_model,
            material: material_handle,
            transform: Transform::from_xyz(3.0, 0.0, 0.0),
            ..default()
        },
        right_shape,
    ));

    commands.spawn((
        Name::new("Point Light"),
        PointLightBundle {
            transform: Transform::from_xyz(4.0, 5.0, 4.0),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Camera"),
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 3.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
    ));
}

fn instructions(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Instructions"),
            NodeBundle {
                style: Style {
                    align_items: AlignItems::Start,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Start,
                    width: Val::Percent(100.),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Space: swap meshes by mutating a Handle<Mesh>",
                TextStyle::default(),
            ));
            parent.spawn(TextBundle::from_section(
                "Return: mutate the mesh itself, changing all copies of it",
                TextStyle::default(),
            ));
        });
}

#[derive(Default)]
struct IsMeshScaled(bool);

fn keyboard_controls(
    asset_server: Res<AssetServer>,
    mut is_mesh_scaled: Local<IsMeshScaled>,
    mut meshes: ResMut<Assets<Mesh>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    left_shape: Query<&Handle<Mesh>, With<Left>>,
    mut right_shape: Query<(&mut Handle<Mesh>, &mut Shape), Without<Left>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        // Mesh handles, like other parts of the ECS, can be queried as mutable and modified at
        // runtime. We only spawned one shape without the `Left` marker component.
        let Ok((mut handle, mut shape)) = right_shape.get_single_mut() else {
            return;
        };

        // Switch to a new Shape variant
        shape.set_next_variant();

        // Modify the handle associated with the Shape on the right side. Note that we will only
        // have to load the same path from storage media once: repeated attempts will re-use the
        // asset.
        *handle = asset_server.load(
            GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset(shape.get_model_path()),
        );
    }

    if keyboard_input.just_pressed(KeyCode::Enter) {
        // It's convenient to retrieve the asset handle stored with the shape on the left. However,
        // we could just as easily have retained this in a resource or a dedicated component.
        let Ok(handle) = left_shape.get_single() else {
            return;
        };

        // Obtain a mutable reference to the Mesh asset.
        let Some(mesh) = meshes.get_mut(handle) else {
            return;
        };

        // Now we can directly manipulate vertices on the mesh. Here, we're just scaling in and out
        // for demonstration purposes. This will affect all entities currently using the asset.
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            // Check a Local value (which only this system can make use of) to determine if we're
            // currently scaled up or not.
            let scale_factor = if is_mesh_scaled.0 { 0.5 } else { 2.0 };

            for position in positions.iter_mut() {
                // Apply the scale factor to each of x, y, and z.
                position[0] *= scale_factor;
                position[1] *= scale_factor;
                position[2] *= scale_factor;
            }

            // Flip the local value to reverse the behaviour next time the key is pressed.
            is_mesh_scaled.0 = !is_mesh_scaled.0;
        }
    }
}
