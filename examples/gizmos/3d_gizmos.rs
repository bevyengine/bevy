//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use std::f32::consts::PI;

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, (draw_example_collection, update_config))
        .run();
}

// We can create our own gizmo config group!
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyRoundGizmos {}

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
        mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
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

    // example instructions
    commands.spawn(
        TextBundle::from_section(
            "Press 'D' to toggle drawing gizmos on top of everything else in the scene\n\
            Press 'P' to toggle perspective for line gizmos\n\
            Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
            Hold 'Up' or 'Down' to change the line width of round gizmos\n\
            Press '1' or '2' to toggle the visibility of straight gizmos or round gizmos\n\
            Press 'A' to show all AABB boxes\n\
            Press 'U' or 'I' to cycle through line styles for straight or round gizmos\n\
            Press 'J' or 'K' to cycle through line joins for straight or round gizmos",
            TextStyle::default(),
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let mut transform = query.single_mut();

    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds() / 2.));
}

fn draw_example_collection(
    mut gizmos: Gizmos,
    mut my_gizmos: Gizmos<MyRoundGizmos>,
    time: Res<Time>,
) {
    gizmos.grid(
        Vec3::ZERO,
        Quat::from_rotation_x(PI / 2.),
        UVec2::splat(20),
        Vec2::new(2., 2.),
        // Light gray
        LinearRgba::gray(0.65),
    );

    gizmos.cuboid(
        Transform::from_translation(Vec3::Y * 0.5).with_scale(Vec3::splat(1.25)),
        BLACK,
    );
    gizmos.rect(
        Vec3::new(time.elapsed_seconds().cos() * 2.5, 1., 0.),
        Quat::from_rotation_y(PI / 2.),
        Vec2::splat(2.),
        LIME,
    );

    gizmos.cross(Vec3::new(-1., 1., 1.), Quat::IDENTITY, 0.5, FUCHSIA);

    my_gizmos.sphere(Vec3::new(1., 0.5, 0.), Quat::IDENTITY, 0.5, RED);

    my_gizmos
        .rounded_cuboid(
            Vec3::new(-2.0, 0.75, -0.75),
            Quat::IDENTITY,
            Vec3::splat(0.9),
            TURQUOISE,
        )
        .edge_radius(0.1)
        .arc_resolution(4);

    for y in [0., 0.5, 1.] {
        gizmos.ray(
            Vec3::new(1., y, 0.),
            Vec3::new(-3., (time.elapsed_seconds() * 3.).sin(), 0.),
            BLUE,
        );
    }

    my_gizmos
        .arc_3d(
            180.0_f32.to_radians(),
            0.2,
            Vec3::ONE,
            Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()),
            ORANGE,
        )
        .resolution(10);

    // Circles have 32 line-segments by default.
    my_gizmos.circle(Vec3::ZERO, Dir3::Y, 3., BLACK);
    // You may want to increase this for larger circles or spheres.
    my_gizmos
        .circle(Vec3::ZERO, Dir3::Y, 3.1, NAVY)
        .resolution(64);
    my_gizmos
        .sphere(Vec3::ZERO, Quat::IDENTITY, 3.2, BLACK)
        .resolution(64);

    gizmos.arrow(Vec3::ZERO, Vec3::ONE * 1.5, YELLOW);

    // You can create more complex arrows using the arrow builder.
    gizmos
        .arrow(Vec3::new(2., 0., 2.), Vec3::new(2., 2., 2.), ORANGE_RED)
        .with_double_end()
        .with_tip_length(0.5);
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if keyboard.just_pressed(KeyCode::KeyD) {
        for (_, config, _) in config_store.iter_mut() {
            config.depth_bias = if config.depth_bias == 0. { -1. } else { 0. };
        }
    }
    if keyboard.just_pressed(KeyCode::KeyP) {
        for (_, config, _) in config_store.iter_mut() {
            // Toggle line_perspective
            config.line_perspective ^= true;
            // Increase the line width when line_perspective is on
            config.line_width *= if config.line_perspective { 5. } else { 1. / 5. };
        }
    }

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    if keyboard.pressed(KeyCode::ArrowRight) {
        config.line_width += 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.line_width -= 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyU) {
        config.line_style = match config.line_style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyJ) {
        config.line_joints = match config.line_joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    let (my_config, _) = config_store.config_mut::<MyRoundGizmos>();
    if keyboard.pressed(KeyCode::ArrowUp) {
        my_config.line_width += 5. * time.delta_seconds();
        my_config.line_width = my_config.line_width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        my_config.line_width -= 5. * time.delta_seconds();
        my_config.line_width = my_config.line_width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        my_config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        my_config.line_style = match my_config.line_style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyK) {
        my_config.line_joints = match my_config.line_joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    if keyboard.just_pressed(KeyCode::KeyA) {
        // AABB gizmos are normally only drawn on entities with a ShowAabbGizmo component
        // We can change this behaviour in the configuration of AabbGizmoGroup
        config_store.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    }
}
