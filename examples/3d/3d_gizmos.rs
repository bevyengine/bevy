//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_internal::{
    gizmos::config::GizmoConfigStore,
    render::{
        settings::{Backends, WgpuSettings},
        RenderPlugin,
    },
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    ..Default::default()
                }
                .into(),
            }),
        )
        .init_gizmo_config::<MyGizmoConfig>()
        .add_systems(Startup, setup)
        .add_systems(Update, (system, rotate_camera, update_config))
        .run();
}

// We can create our own gizmo config!
#[derive(Resource, Default)]
struct MyGizmoConfig {}

impl CustomGizmoConfig for MyGizmoConfig {}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 1.5, 6.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane::from_size(5.0))),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // example instructions
    commands.spawn(
        TextBundle::from_section(
            "Press 'D' to toggle drawing gizmos on top of everything else in the scene\n\
            Press 'P' to toggle perspective for line gizmos\n\
            Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
            Hold 'Up' or 'Down' to change the line width of round gizmos\n\
            Press '1' or '2' to toggle the visibility of straight gizmos or round gizmos\n\
            Press 'A' to show all AABB boxes",
            TextStyle {
                font_size: 20.,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn system(mut gizmos: Gizmos, mut my_gizmos: Gizmos<MyGizmoConfig>, time: Res<Time>) {
    gizmos.cuboid(
        Transform::from_translation(Vec3::Y * 0.5).with_scale(Vec3::splat(1.25)),
        Color::BLACK,
    );
    gizmos.rect(
        Vec3::new(time.elapsed_seconds().cos() * 2.5, 1., 0.),
        Quat::from_rotation_y(PI / 2.),
        Vec2::splat(2.),
        Color::GREEN,
    );

    my_gizmos.sphere(Vec3::new(1., 0.5, 0.), Quat::IDENTITY, 0.5, Color::RED);

    for y in [0., 0.5, 1.] {
        gizmos.ray(
            Vec3::new(1., y, 0.),
            Vec3::new(-3., (time.elapsed_seconds() * 3.).sin(), 0.),
            Color::BLUE,
        );
    }

    // Circles have 32 line-segments by default.
    my_gizmos.circle(Vec3::ZERO, Vec3::Y, 3., Color::BLACK);
    // You may want to increase this for larger circles or spheres.
    my_gizmos
        .circle(Vec3::ZERO, Vec3::Y, 3.1, Color::NAVY)
        .segments(64);
    my_gizmos
        .sphere(Vec3::ZERO, Quat::IDENTITY, 3.2, Color::BLACK)
        .circle_segments(64);
}

fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let mut transform = query.single_mut();

    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds() / 2.));
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    mut aabb_config: ResMut<AabbGizmoConfig>,
    keyboard: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let config = config_store.get_mut::<DefaultGizmoConfig>();
    if keyboard.just_pressed(KeyCode::D) {
        config.depth_bias = if config.depth_bias == 0. { -1. } else { 0. };
    }
    if keyboard.just_pressed(KeyCode::P) {
        // Toggle line_perspective
        config.line_perspective ^= true;
        // Increase the line width when line_perspective is on
        config.line_width *= if config.line_perspective { 5. } else { 1. / 5. };
    }

    if keyboard.pressed(KeyCode::Right) {
        config.line_width += 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::Left) {
        config.line_width -= 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Key1) {
        config.enabled ^= true;
    }

    let my_config = config_store.get_mut::<MyGizmoConfig>();
    if keyboard.just_pressed(KeyCode::D) {
        my_config.depth_bias = if my_config.depth_bias == 0. { -1. } else { 0. };
    }
    if keyboard.just_pressed(KeyCode::P) {
        // Toggle line_perspective
        my_config.line_perspective ^= true;
        // Increase the line width when line_perspective is on
        my_config.line_width *= if my_config.line_perspective {
            5.
        } else {
            1. / 5.
        };
    }

    if keyboard.pressed(KeyCode::Up) {
        my_config.line_width += 5. * time.delta_seconds();
        my_config.line_width = my_config.line_width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::Down) {
        my_config.line_width -= 5. * time.delta_seconds();
        my_config.line_width = my_config.line_width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Key2) {
        my_config.enabled ^= true;
    }

    if keyboard.just_pressed(KeyCode::A) {
        // AABB gizmos are normally only drawn on entities with a ShowAabbGizmo component
        // We can change this behaviour in the extended configuration of AabbGizmos config
        aabb_config.draw_all ^= true;
    }
}
