//! A simple 3D scene with light shining over a cube sitting on a plane. (todo)

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

/// Marker for the health bar root UI node
#[derive(Component)]
struct HealthBarMarker;

/// Health bar should be moved above this entity (todo: use relations)
#[derive(Component)]
struct HealthBarTarget;

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
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(Update, update_ui.run_if(input_just_pressed(KeyCode::Space)))
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
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_translation(entity_spawn),
        HealthBarTarget,
        Movable::new(entity_spawn)
    ));
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
        Transform::from_xyz(-4.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// todo comment
fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(30.0),
            top: Val::Vw(50.0),
            width: Val::Px(150.0),
            height: Val::Px(65.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.5, 0.1, 0.3)),
        HealthBarMarker,
        children![(
            Text::new("42"),
            TextFont {
                font: font_handle.clone(),
                font_size: 33.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
        )],
    ));
}

fn update_ui(
    mut health_bar_query: Query<&mut Node, With<HealthBarMarker>>,
    target_query: Single<&GlobalTransform, With<HealthBarTarget>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
) {
    let camera = camera_query.0;
    let cam_transform = camera_query.1;

    let world_position = target_query.translation();

    for mut health_bar_node in health_bar_query.iter_mut() {
        let viewport_position = camera.world_to_viewport(cam_transform, world_position).unwrap();
        health_bar_node.left = Val::Px(viewport_position.x);
        health_bar_node.top = Val::Px(viewport_position.y);

        println!("{}", camera.world_to_viewport(cam_transform, Vec3::ZERO).unwrap());
        println!("{:?}", health_bar_node.left);
        println!("{:?}", health_bar_node.top);
    }
}

// This system will move all Movable entities with a Transform
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

