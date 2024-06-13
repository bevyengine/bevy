use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (spawn_view_model, spawn_world_model))
        .add_systems(Update, move_player)
        .run();
}

#[derive(Debug, Component)]
struct Player;

fn spawn_view_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arm = meshes.add(Cuboid::new(0.1, 0.1, 0.8));
    let arm_material = materials.add(Color::srgb(0.5, 0.5, 1.0));

    commands
        .spawn((
            Name::new("Player"),
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 1.0, 0.0),
                ..default()
            },
            Player,
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("View Model Camera"),
                Camera3dBundle { ..default() },
            ));

            parent.spawn((
                Name::new("Arm"),
                MaterialMeshBundle {
                    mesh: arm,
                    material: arm_material,
                    transform: Transform::from_xyz(0.2, -0.1, -0.4),
                    ..default()
                },
            ));
        });
}

fn spawn_world_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)));
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material = materials.add(Color::srgb(1.0, 0.5, 0.5));

    commands.spawn(
        (MaterialMeshBundle {
            mesh: floor,
            material: material.clone(),
            ..default()
        }),
    );

    commands.spawn(
        (MaterialMeshBundle {
            mesh: cube,
            material,
            transform: Transform::from_xyz(0.0, 0.0, -3.0),
            ..default()
        }),
    );

    commands.spawn(
        (PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        }),
    );
}

fn move_player(
    mut mouse_motion: EventReader<MouseMotion>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    let mut transform = player.single_mut();
    for motion in mouse_motion.read() {
        let yaw = -motion.delta.x * 0.002;
        let pitch = -motion.delta.y * 0.002;
        transform.rotate_y(yaw);
        transform.rotate_local_x(pitch);
    }
}
