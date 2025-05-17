//! A simple 3D scene with light shining over a cube sitting on a plane. (todo)

use bevy::color::palettes::css::{GREEN, RED};
use bevy::prelude::*;

const BAR_HEIGHT: f32 = 15.0;
const BAR_WIDTH: f32 = 150.0;
const HALF_BAR_HEIGHT: f32 = BAR_HEIGHT / 2.0;
const HALF_BAR_WIDTH: f32 = BAR_WIDTH / 2.0;

#[derive(Component)]
struct HealthBar {
    /// The target entity that the health bar should follow
    target: Entity,
    /// The root UI node used to position the health bar
    root_node: Entity,
}

// Define a struct to keep some information about our entity.
// Here it's an arbitrary movement speed, the spawn location, and a maximum distance from it.
#[derive(Component)]
struct Movable {
    spawn: Vec3,
    max_distance: f32,
    speed: f32,
}

// Implement a utility function for easier Movable struct creation.
impl Movable {
    fn new(spawn: Vec3) -> Self {
        Movable {
            spawn,
            max_distance: 5.0,
            speed: 2.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_ui)
        .add_systems(Update, move_cube)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    let entity_spawn = Vec3::new(0.0, 0.5, 0.0);
    let cube_id = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_translation(entity_spawn),
            Movable::new(entity_spawn),
        ))
        .id();
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.5, 2.5, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let health_bar_root = commands
        .spawn((
            Name::from("Root Healthbar"),
            Node {
                width: Val::Px(BAR_WIDTH),
                height: Val::Px(BAR_HEIGHT),
                padding: UiRect::all(Val::Px(4.)),
                display: Display::Flex,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            children![()],
        ))
        .id();

    let health_bar_nodes = commands
        .spawn((
            Node {
                align_items: AlignItems::Stretch,
                width: Val::Percent(100.),
                ..default()
            },
            BackgroundColor(Color::from(RED)),
            children![(
                Node::default(),
                BackgroundColor(Color::from(GREEN)),
                HealthBar {
                    target: cube_id,
                    root_node: health_bar_root,
                }
            )],
        ))
        .id();

    commands.entity(health_bar_root).add_child(health_bar_nodes);
}

fn update_ui(
    mut health_bar_query: Query<(&mut Node, &HealthBar)>,
    mut health_bar_root_query: Query<&mut Node, Without<HealthBar>>,
    target_query: Query<&GlobalTransform>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
) {
    let camera = camera_query.0;
    let cam_transform = camera_query.1;

    for (mut health_bar_node, health_bar_component) in health_bar_query.iter_mut() {
        let mut root = health_bar_root_query
            .get_mut(health_bar_component.root_node)
            .unwrap();
        let target = target_query.get(health_bar_component.target).unwrap();
        let world_position = target.translation();

        let viewport_position = camera
            .world_to_viewport(cam_transform, world_position)
            .unwrap();
        root.left = Val::Px(viewport_position.x - HALF_BAR_WIDTH);
        root.top = Val::Px(viewport_position.y - HALF_BAR_HEIGHT);

        let hp = (time.elapsed().as_secs_f32().sin() + 0.5) * 100.0;
        health_bar_node.width = Val::Percent(hp);
    }
}

/// This system will move all Movable entities with a Transform
fn move_cube(mut cubes: Query<(&mut Transform, &mut Movable)>, timer: Res<Time>) {
    for (mut transform, mut cube) in &mut cubes {
        // Check if the entity moved too far from its spawn, if so invert the moving direction.
        if (cube.spawn - transform.translation).length() > cube.max_distance {
            cube.speed *= -1.0;
        }
        let direction = transform.local_x();
        transform.translation += direction * cube.speed * timer.delta_secs();
    }
}
