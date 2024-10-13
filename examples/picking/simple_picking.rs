//! A simple scene to demonstrate picking events

use bevy::{color::palettes::tailwind::CYAN_400, prelude::*};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    app.add_systems(Startup, setup);

    app.run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((
            Text::new("Click Me to get a box"),
            Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(12.0),
                left: Val::Percent(12.0),
                ..default()
            },
        ))
        .observe(
            |_click: Trigger<Pointer<Click>>,
             mut commands: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>,
             mut num: Local<usize>| {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                    MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
                    Transform::from_xyz(0.0, 0.5 + 1.1 * *num as f32, 0.0),
                ));
                *num += 1;
            },
        )
        .observe(
            |evt: Trigger<Pointer<Out>>, mut texts: Query<&mut TextStyle>| {
                let mut style = texts.get_mut(evt.entity()).unwrap();
                style.color = Color::WHITE;
            },
        )
        .observe(
            |evt: Trigger<Pointer<Over>>, mut texts: Query<&mut TextStyle>| {
                let mut style = texts.get_mut(evt.entity()).unwrap();
                style.color = CYAN_400.into();
            },
        );
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
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
