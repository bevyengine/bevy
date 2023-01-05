//! Illustrates the difference between direction of a translation in respect to local object or
//! global object Transform.

use bevy::{math::Vec3A, prelude::*};

// Define a marker for entities that should be changed via their global transform.
#[derive(Component)]
struct ChangeGlobal;

// Define a marker for entities that should be changed via their local transform.
#[derive(Component)]
struct ChangeLocal;

// Define a marker for entities that should move.
#[derive(Component)]
struct Move;

// Define a resource for the current movement direction;
#[derive(Resource, Default)]
struct Direction(Vec3);

// Define component to decide when an entity should be ignored by the movement systems.
#[derive(Component)]
struct ToggledBy(KeyCode);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .init_resource::<Direction>()
        .add_system(move_cubes_according_to_global_transform)
        .add_system(move_cubes_according_to_local_transform)
        .add_system(update_directional_input)
        .add_system(toggle_movement)
        .run();
}

// Startup system to setup the scene and spawn all relevant entities.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // To show the difference between a local transform (rotation, scale and position in respect to a given entity)
    // and global transform (rotation, scale and position in respect to the base coordinate system of the visible scene)
    // it's helpful to add multiple entities that are attached to each other.
    // This way we'll see that the transform in respect to an entity's parent is different to the
    // global transform within the visible scene.
    // This example focuses on translation only to clearly demonstrate the differences.

    // Spawn a basic cube to have an entity as reference.
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                material: materials.add(StandardMaterial {
                    base_color: Color::YELLOW,
                    alpha_mode: AlphaMode::Blend,
                    ..Default::default()
                }),
                ..default()
            },
            ChangeGlobal,
            Move,
            ToggledBy(KeyCode::Key1),
        ))
        // Spawn two entities as children above the original main entity.
        // The red entity spawned here will be changed via its global transform
        // where the green one will be changed via its local transform.
        .with_children(|child_builder| {
            // also see parenting example
            child_builder.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::RED,
                        alpha_mode: AlphaMode::Blend,
                        ..Default::default()
                    }),
                    transform: Transform::from_translation(Vec3::Y - Vec3::Z),
                    ..default()
                },
                ChangeGlobal,
                Move,
                ToggledBy(KeyCode::Key2),
            ));
            child_builder.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::GREEN,
                        alpha_mode: AlphaMode::Blend,
                        ..Default::default()
                    }),
                    transform: Transform::from_translation(Vec3::Y + Vec3::Z),
                    ..default()
                },
                ChangeLocal,
                Move,
                ToggledBy(KeyCode::Key3),
            ));
        });

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add a light source for better 3d visibility.
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::splat(3.0)),
        ..default()
    });

    // Add text to explain inputs and what is happening.
    commands.spawn(TextBundle::from_section(
        "Press the arrow keys to move the cubes. Toggle movement for yellow (1), red (2) and green (3) cubes via number keys.

Notice how the green cube will translate further in respect to the yellow in contrast to the red cube.
This is due to the use of its LocalTransform that is relative to the yellow cubes transform instead of the GlobalTransform as in the case of the red cube.
The red cube is moved through its GlobalTransform and thus is unaffected by the yellows transform.",
        TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 22.0,
            color: Color::WHITE,
        },
    ));
}

// This system will move all cubes that are marked as ChangeGlobal according to their global transform.
fn move_cubes_according_to_global_transform(
    mut cubes: Query<&mut GlobalTransform, (With<ChangeGlobal>, With<Move>)>,
    direction: Res<Direction>,
    timer: Res<Time>,
) {
    for mut global_transform in &mut cubes {
        *global_transform.translation_mut() += Vec3A::from(direction.0) * timer.delta_seconds();
    }
}

// This system will move all cubes that are marked as ChangeLocal according to their local transform.
fn move_cubes_according_to_local_transform(
    mut cubes: Query<&mut Transform, (With<ChangeLocal>, With<Move>)>,
    direction: Res<Direction>,
    timer: Res<Time>,
) {
    for mut transform in &mut cubes {
        transform.translation += direction.0 * timer.delta_seconds();
    }
}

// This system updates a resource that defines in which direction the cubes should move.
// The direction is defined by the input of arrow keys and is only in left/right and up/down direction.
fn update_directional_input(mut direction: ResMut<Direction>, keyboard_input: Res<Input<KeyCode>>) {
    let horizontal_movement = Vec3::X
        * (keyboard_input.pressed(KeyCode::Right) as i32
            - keyboard_input.pressed(KeyCode::Left) as i32) as f32;
    let vertical_movement = Vec3::Y
        * (keyboard_input.pressed(KeyCode::Up) as i32
            - keyboard_input.pressed(KeyCode::Down) as i32) as f32;
    direction.0 = horizontal_movement + vertical_movement;
}

// This system enables and disables the movement for each entity if their assigned key is pressed.
fn toggle_movement(
    mut commands: Commands,
    movable_entities: Query<(Entity, &Handle<StandardMaterial>, &ToggledBy), With<Move>>,
    static_entities: Query<(Entity, &Handle<StandardMaterial>, &ToggledBy), Without<Move>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    // Update the currently movable entities and remove their Move component if the assigned key was pressed to disable their movement.
    // This will also make them transparent so they can be identified as 'disabled' in the scene.
    for (entity, material_handle, toggled_by) in &movable_entities {
        if keyboard_input.just_pressed(toggled_by.0) {
            materials
                .get_mut(material_handle)
                .unwrap()
                .base_color
                .set_a(0.5);
            commands.entity(entity).remove::<Move>();
        }
    }
    // Update the currently non-movable entities and add a Move component if the assigned key was pressed to enable their movement.
    // This will also make them opaque so they can be identified as 'enabled' in the scene.
    for (entity, material_handle, toggled_by) in &static_entities {
        if keyboard_input.just_pressed(toggled_by.0) {
            materials
                .get_mut(material_handle)
                .unwrap()
                .base_color
                .set_a(1.0);
            commands.entity(entity).insert(Move);
        }
    }
}
