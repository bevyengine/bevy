use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (spawn_view_model, spawn_world_model, spawn_text))
        .add_systems(Update, (move_player, change_fov))
        .run();
}

#[derive(Debug, Component)]
struct Player;
#[derive(Debug, Component)]
struct WorldModelCamera;

// Setup systems

fn spawn_view_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arm = meshes.add(Cuboid::new(0.1, 0.1, 0.5));
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
                Name::new("World Model Camera"),
                Camera3dBundle {
                    projection: PerspectiveProjection {
                        fov: 90.0_f32.to_radians(),
                        ..default()
                    }
                    .into(),
                    ..default()
                },
                WorldModelCamera,
            ));
            parent.spawn((
                Name::new("View Model Camera"),
                Camera3dBundle {
                    camera: Camera {
                        order: 1,
                        ..default()
                    },
                    projection: PerspectiveProjection {
                        fov: 70.0_f32.to_radians(),
                        ..default()
                    }
                    .into(),
                    ..default()
                },
                RenderLayers::layer(1),
            ));

            parent.spawn((
                Name::new("Arm"),
                MaterialMeshBundle {
                    mesh: arm,
                    material: arm_material,
                    transform: Transform::from_xyz(0.2, -0.1, -0.25),
                    ..default()
                },
                RenderLayers::layer(1),
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

    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        },
        RenderLayers::from_layers(&[0, 1]),
    ));
}

fn spawn_text(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                concat!(
                    "Move the camera with your mouse.\n",
                    "Press arrow up to decrease the FOV of the world model.\n",
                    "Press arrow down to increase the FOV of the world model."
                ),
                TextStyle {
                    font_size: 25.0,
                    ..default()
                },
            ));
        });
}

// Functional systems

fn move_player(
    mut mouse_motion: EventReader<MouseMotion>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    let mut transform = player.single_mut();
    for motion in mouse_motion.read() {
        let yaw = -motion.delta.x * 0.003;
        let pitch = -motion.delta.y * 0.002;
        transform.rotate_y(yaw);
        transform.rotate_local_x(pitch);
    }
}

fn change_fov(
    input: Res<ButtonInput<KeyCode>>,
    mut world_model_projection: Query<&mut Projection, With<WorldModelCamera>>,
) {
    let mut projection = world_model_projection.single_mut();
    let Projection::Perspective(ref mut projection) = projection.as_mut() else {
        unreachable!();
    };

    if input.pressed(KeyCode::ArrowUp) {
        projection.fov -= 1.0_f32.to_radians();
        projection.fov = projection.fov.max(20.0_f32.to_radians());
    }
    if input.pressed(KeyCode::ArrowDown) {
        projection.fov += 1.0_f32.to_radians();
        projection.fov = projection.fov.min(160.0_f32.to_radians());
    }
}
