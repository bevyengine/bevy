//! A simple scene to demonstrate picking events for UI and mesh entities.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((
            Text::new("Click Me to get a box\nDrag cubes to rotate"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(12.0),
                left: Val::Percent(12.0),
                ..default()
            },
        ))
        .observe(on_click_spawn_cube)
        .observe(|out: On<Pointer<Out>>, mut texts: Query<&mut TextColor>| {
            let mut text_color = texts.get_mut(out.target()).unwrap();
            text_color.0 = Color::WHITE;
        })
        .observe(
            |over: On<Pointer<Over>>, mut texts: Query<&mut TextColor>| {
                let mut color = texts.get_mut(over.target()).unwrap();
                color.0 = bevy::color::palettes::tailwind::CYAN_400.into();
            },
        );

    // Base
    commands.spawn((
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
    if let Ok(mut transform) = transforms.get_mut(drag.target()) {
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
    }
}
