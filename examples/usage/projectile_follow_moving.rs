//! Shows how to create moving and spining objects on mouse position.
//!
//! Left click on with the mouse will spawn a projectile that follows the moving target.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};

const MIN_DISTANCE: f32 = 10.0;

/// Represents a movable entity with a spawn location, movement speed, and a maximum allowed distance from its spawn point.
///
/// This component is later used by `Follower` projectiles to select and follow a target.
#[derive(Component, Debug)]
struct Movable {
    spawn: Vec3,
    max_distance: f32,
    speed: f32,
}

/// Utility constructor for creating a `Movable` with default speed and max distance values.
impl Movable {
    fn new(spawn: Vec3) -> Self {
        Movable {
            spawn,
            max_distance: 300.0,
            speed: 70.0,
        }
    }
}

/// Marks a projectile that follows a target entity, using a specified speed.
///
/// - `target`: An `Entity` identifier used to reference the target in queries.
/// - `speed`: The speed at which the projectile moves toward the target.
#[derive(Component)]
struct Follower {
    speed: f32,
    target: Entity,
}

impl Follower {
    /// Creates a new `Follower` with a default speed and the given target entity.
    fn new(target: Entity) -> Self {
        Follower {
            speed: 110.0,
            target,
        }
    }
}

/// A resource that holds the mesh and material used for rendering 2D projectiles.
#[derive(Resource)]
struct WorldAssets {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, move_target)
        .add_systems(
            Update,
            spawn_projectile.run_if(input_just_pressed(MouseButton::Left)),
        )
        .add_systems(
            Update,
            move_projectile.run_if(any_with_component::<Movable>),
        )
        .run();
}

// Startup system to setup the scene and spawn all relevant entities.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Create mesh handle and material for projectile.
    let projectile_asset = WorldAssets {
        mesh: meshes.add(Triangle2d::new(
            Vec2::Y * 5.0,
            Vec2::new(-5.0, -5.0),
            Vec2::new(5.0, -5.0),
        )),
        material: materials.add(Color::from(bevy::color::palettes::basic::RED)),
    };

    // Add handler of triabgle and it's material as Resource.
    commands.insert_resource(projectile_asset);

    // Add a shape as Movable.
    let entity_spawn = Vec3::ZERO;
    commands.spawn((
        Mesh2d(meshes.add(RegularPolygon::new(30.0, 8))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::from_translation(entity_spawn),
        Movable::new(entity_spawn),
    ));

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn((Camera2d, Transform::from_xyz(0.0, 0.0, 0.0)));

    // this creates a Text comment and the Node is need as a Ui description to spawn it in the top left corner
    commands.spawn((
        Text::new("Mouseclick: to fire projectile from mouse position towards moving target."),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Spawns a projectile at the current mouse cursor position and assigns the only `Movable` entity as its target.
///
/// The function performs the following steps:
/// 1. Calculates the cursor position relative to the world by using the current window and camera viewport.
/// 2. Retrieves the first and only entity with the `Movable` component to be used as the projectile's target.
/// 3. Spawns the projectile at the computed world position using the `WorldAsset` resource created during setup.
fn spawn_projectile(
    mut commands: Commands,
    world_assets: Res<WorldAssets>,
    targets: Single<(Entity, &mut Movable)>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
) -> Result<(), BevyError> {
    let (camera, camera_transform) = *camera_query;
    let window = window.single()?;

    let Some(cursor_position) = window.cursor_position() else {
        return Ok(());
    };

    // Calculate a world position based on the cursor's position.
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return Ok(());
    };

    let entity_spawn = Vec3::new(world_pos.x, world_pos.y, 0.0);

    let target_entity = targets.0;
    commands.spawn((
        Mesh2d(world_assets.mesh.clone()),
        MeshMaterial2d(world_assets.material.clone()),
        Transform::from_translation(entity_spawn),
        Follower::new(target_entity),
    ));

    Ok(())
}

/// This system will move all Movable entities with a Transform
fn move_target(mut targets: Query<(&mut Transform, &mut Movable)>, timer: Res<Time>) {
    for (mut transform, mut target) in &mut targets {
        // Check if the entity moved too far from its spawn, if so invert the moving direction.
        if (target.spawn - transform.translation).length() > target.max_distance {
            target.speed *= -1.0;
        }

        // the direction will be horizontal to window. local_x of transform is cosntantly changing due to rotation of z-axis of transform.
        let direction = Dir3::X;
        transform.translation += direction * target.speed * timer.delta_secs();

        // this makes the polygon look like roling.
        transform.rotate_z(-1.2 * target.speed.signum() * timer.delta_secs());
    }
}

/// Moves each projectile toward its target and despawns it once it gets close enough.
///
/// For each projectile:
/// - Updates its position to move closer to its assigned target.
/// - Checks the distance to the target, and if it is within a threshold, the projectile is despawned.
fn move_projectile(
    mut projectiles: Query<(Entity, &mut Transform, &mut Follower)>,
    targets: Query<&Transform, Without<Follower>>,
    mut commands: Commands,
    timer: Res<Time>,
) {
    for (entity, mut transform, projectile) in &mut projectiles {
        let target_transform = targets.get(projectile.target).unwrap();

        // the direction and distance of the target.
        let vector_to_target = target_transform.translation - transform.translation;

        // if the projectile is close enough, despawn it
        if vector_to_target.length() < MIN_DISTANCE {
            commands.entity(entity).despawn();
        }

        // normalized vector_to_target is the direction.
        let direction = vector_to_target.normalize();

        transform.translation += direction * projectile.speed * timer.delta_secs();

        // rotate to point to target
        let rotate_to_target = Quat::from_rotation_arc(Vec3::Y, direction.xy().extend(0.));
        transform.rotation = rotate_to_target;
    }
}
