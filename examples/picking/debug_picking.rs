//! A simple scene to demonstrate picking events for UI and mesh entities,
//! Demonstrates how to change debug settings

use bevy::dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            filter: "bevy_dev_tools=trace".into(), // Show picking logs trace level and up
            ..default()
        }))
        .add_plugins((MeshPickingPlugin, DebugPickingPlugin))
        .add_systems(Startup, setup_scene)
        .insert_resource(DebugPickingMode::Normal)
        // A system that cycles the debugging state when you press F3:
        .add_systems(
            PreUpdate,
            (|mut mode: ResMut<DebugPickingMode>| {
                *mode = match *mode {
                    DebugPickingMode::Disabled => DebugPickingMode::Normal,
                    DebugPickingMode::Normal => DebugPickingMode::Noisy,
                    DebugPickingMode::Noisy => DebugPickingMode::Disabled,
                }
            })
            .distributive_run_if(bevy::input::common_conditions::input_just_pressed(
                KeyCode::F3,
            )),
        )
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((
            Text::new("Click Me to get a box\nDrag cubes to rotate\nPress F3 to cycle between picking debug levels"),
            Node {
                position_type: PositionType::Absolute,
                top: percent(12),
                left: percent(12),
                ..default()
            },
        ))
        .observe(on_click_spawn_cube)
        .observe(
            |out: On<Pointer<Out>>, mut texts: Query<&mut TextColor>| {
                let mut text_color = texts.get_mut(out.entity).unwrap();
                text_color.0 = Color::WHITE;
            },
        )
        .observe(
            |over: On<Pointer<Over>>, mut texts: Query<&mut TextColor>| {
                let mut color = texts.get_mut(over.entity).unwrap();
                color.0 = bevy::color::palettes::tailwind::CYAN_400.into();
            },
        );

    // Base
    commands.spawn((
        Name::new("Base"),
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn on_click_spawn_cube(
    _click: On<Pointer<Click>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut num: Local<usize>,
) {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_xyz(0.0, 0.25 + 0.55 * *num as f32, 0.0),
        ))
        // With the MeshPickingPlugin added, you can add pointer event observers to meshes:
        .observe(on_drag_rotate);
    *num += 1;
}

fn on_drag_rotate(drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    if let Ok(mut transform) = transforms.get_mut(drag.entity) {
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
    }
}
