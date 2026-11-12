//! Test that a mesh is still drawn after its `Mesh3d` is removed and added again.

use bevy::{diagnostic::FrameCount, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Test Mesh Reinsert".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, swap_meshes)
        .run();
}

/// Length of one remove-and-add cycle, in frames.
const CYCLE: u32 = 120;
/// Frame within a cycle at which the meshes are removed.
const REMOVE_AT: u32 = 40;
/// Frame within a cycle at which the [`Later`] mesh is added again.
const ADD_AT: u32 = 80;

/// Loses its `Mesh3d` at [`REMOVE_AT`] and gets a new one at [`ADD_AT`].
#[derive(Component)]
struct Later;

/// Loses its `Mesh3d` and gets a new one in the same frame, at [`REMOVE_AT`].
#[derive(Component)]
struct SameFrame;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let description = "(left to right)\n\
        0: Mesh3d never removed.\n\
        1: Mesh3d removed, then added again 40 frames later (blinks).\n\
        2: Mesh3d removed and added again in the same frame (always visible).";
    commands.spawn((
        Text::new(description),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    let cube = meshes.add(Cuboid::default());
    let material = materials.add(Color::srgb(0.8, 0.7, 0.6));
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(-2.0, 0.0, 0.0),
    ));
    commands.spawn((
        Later,
        Mesh3d(cube.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    commands.spawn((
        SameFrame,
        Mesh3d(cube),
        MeshMaterial3d(material),
        Transform::from_xyz(2.0, 0.0, 0.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 8.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn swap_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    frame: Res<FrameCount>,
    later: Single<Entity, With<Later>>,
    same_frame: Single<Entity, With<SameFrame>>,
) {
    match frame.0 % CYCLE {
        REMOVE_AT => {
            commands.entity(*later).remove::<Mesh3d>();
            // A new mesh asset, as when a mesh is rebuilt.
            commands
                .entity(*same_frame)
                .remove::<Mesh3d>()
                .insert(Mesh3d(meshes.add(Cuboid::default())));
        }
        ADD_AT => {
            commands
                .entity(*later)
                .insert(Mesh3d(meshes.add(Cuboid::default())));
        }
        _ => {}
    }
}
