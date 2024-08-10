//! A simple scene to demonstrate picking events

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((
            TextBundle {
                text: Text::from_section("Click Me to get a box", TextStyle::default()),
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(10.0),
                    left: Val::Percent(10.0),
                    ..default()
                },
                ..Default::default()
            },
            Pickable::default(),
        ))
        .observe(
            |_click: Trigger<Pointer<Click>>,
             mut commands: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>,
             mut num: Local<usize>| {
                commands.spawn(PbrBundle {
                    mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                    material: materials.add(Color::srgb_u8(124, 144, 255)),
                    transform: Transform::from_xyz(0.0, 0.5 + 1.1 * *num as f32, 0.0),
                    ..default()
                });
                *num += 1;
            },
        )
        .observe(|evt: Trigger<Pointer<Out>>, mut texts: Query<&mut Text>| {
            let mut text = texts.get_mut(evt.entity()).unwrap();
            let first = text.sections.first_mut().unwrap();
            first.style.color = WHITE.into();
        })
        .observe(|evt: Trigger<Pointer<Over>>, mut texts: Query<&mut Text>| {
            let mut text = texts.get_mut(evt.entity()).unwrap();
            let first = text.sections.first_mut().unwrap();
            first.style.color = BLUE.into();
        });
    // circular base
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Circle::new(4.0)),
                material: materials.add(Color::WHITE),
                transform: Transform::from_rotation(Quat::from_rotation_x(
                    -std::f32::consts::FRAC_PI_2,
                )),
                ..default()
            },
            Pickable::default(),
        ))
        .observe(|click: Trigger<Pointer<Click>>| {
            let click = click.event();
            println!("{click:?}");
        });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
