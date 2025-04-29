//! A simple 3D scene to demonstrate parent picking.
//!
//! Entity hierachies can be built using a [`ChildOf`] component.  By observing for
//! [`Trigger<Pointer<E>>`] events on the parent entity, picking events can be collected
//! for the entire tree, giving both the leaf and root nodes selected.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, close_on_esc)

        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let parent_id = commands.spawn((
        Name::new("Parent"),
        Transform::IDENTITY,
        Visibility::Visible,
    ))
    .observe(on_pointer_over_debug)
    .id();


    // circular base
    commands.spawn((
        Name::new("Base"),
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ChildOf(parent_id),
    ));

    // cube
    commands.spawn((
        Name::new("Cube"),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        ChildOf(parent_id),
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
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn on_pointer_over_debug(
    trigger: Trigger<Pointer<Over>>,
    query2: Query<& Name>,
) {
    if let Ok(name) = query2.get(trigger.target()) {
        println!("root = trigger.target() = {:?}", name);
    }

    // NOTE: trigger.original_target = trigger.event().original_target
    // this is due to [`impl Deref for Pointer`]
    if let Ok(name) = query2.get(trigger.original_target) {
        
        println!("leaf = trigger.original_target = {:?}", name);
    }
}

fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}
