//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{color::palettes::css::*, prelude::*};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CameraControllerPlugin))
        .init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_example_collection, update_config))
        .run();
}

// We can create our own gizmo config group!
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyRoundGizmos;

fn setup(
    mut commands: Commands,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut gizmo = GizmoAsset::new();

    // When drawing a lot of static lines a Gizmo component can have
    // far better performance than the Gizmos system parameter,
    // but the system parameter will perform better for smaller lines that update often.

    // A sphere made out of 30_000 lines!
    gizmo
        .sphere(Isometry3d::IDENTITY, 0.5, CRIMSON)
        .resolution(30_000 / 3);

    commands.spawn((
        Gizmo {
            handle: gizmo_assets.add(gizmo),
            line_config: GizmoLineConfig {
                width: 5.,
                ..default()
            },
            ..default()
        },
        Transform::from_xyz(4., 1., 0.),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1.5, 6.).looking_at(Vec3::ZERO, Vec3::Y),
        CameraController::default(),
    ));
    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // example instructions
    commands.spawn((
        Text::new(
            "Press 'T' to toggle drawing gizmos on top of everything else in the scene\n\
            Press 'P' to toggle perspective for line gizmos\n\
            Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
            Hold 'Up' or 'Down' to change the line width of round gizmos\n\
            Press '1' or '2' to toggle the visibility of straight gizmos or round gizmos\n\
            Press 'B' to show all AABB boxes\n\
            Press 'U' or 'I' to cycle through line styles for straight or round gizmos\n\
            Press 'J' or 'K' to cycle through line joins for straight or round gizmos",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn draw_example_collection(
    mut gizmos: Gizmos,
    mut my_gizmos: Gizmos<MyRoundGizmos>,
    time: Res<Time>,
) {
    gizmos.grid(
        Quat::from_rotation_x(PI / 2.),
        UVec2::splat(20),
        Vec2::new(2., 2.),
        // Light gray
        LinearRgba::gray(0.65),
    );
    gizmos.grid(
        Isometry3d::new(Vec3::splat(10.0), Quat::from_rotation_x(PI / 3. * 2.)),
        UVec2::splat(20),
        Vec2::new(2., 2.),
        PURPLE,
    );
    gizmos.sphere(Vec3::splat(10.0), 1.0, PURPLE);

    gizmos
        .primitive_3d(
            &Plane3d {
                normal: Dir3::Y,
                half_size: Vec2::splat(1.0),
            },
            Isometry3d::new(
                Vec3::splat(4.0) + Vec2::from(ops::sin_cos(time.elapsed_secs())).extend(0.0),
                Quat::from_rotation_x(PI / 2. + time.elapsed_secs()),
            ),
            GREEN,
        )
        .cell_count(UVec2::new(5, 10))
        .spacing(Vec2::new(0.2, 0.1));

    gizmos.cuboid(
        Transform::from_translation(Vec3::Y * 0.5).with_scale(Vec3::splat(1.25)),
        BLACK,
    );
    gizmos.rect(
        Isometry3d::new(
            Vec3::new(ops::cos(time.elapsed_secs()) * 2.5, 1., 0.),
            Quat::from_rotation_y(PI / 2.),
        ),
        Vec2::splat(2.),
        LIME,
    );

    gizmos.cross(Vec3::new(-1., 1., 1.), 0.5, FUCHSIA);

    let domain = Interval::EVERYWHERE;
    let curve = FunctionCurve::new(domain, |t| {
        (Vec2::from(ops::sin_cos(t * 10.0))).extend(t - 6.0)
    });
    let resolution = ((ops::sin(time.elapsed_secs()) + 1.0) * 100.0) as usize;
    let times_and_colors = (0..=resolution)
        .map(|n| n as f32 / resolution as f32)
        .map(|t| t * 5.0)
        .map(|t| (t, TEAL.mix(&HOT_PINK, t / 5.0)));
    gizmos.curve_gradient_3d(curve, times_and_colors);

    my_gizmos.sphere(Vec3::new(1., 0.5, 0.), 0.5, RED);

    my_gizmos
        .rounded_cuboid(Vec3::new(-2.0, 0.75, -0.75), Vec3::splat(0.9), TURQUOISE)
        .edge_radius(0.1)
        .arc_resolution(4);

    for y in [0., 0.5, 1.] {
        gizmos.ray(
            Vec3::new(1., y, 0.),
            Vec3::new(-3., ops::sin(time.elapsed_secs() * 3.), 0.),
            BLUE,
        );
    }

    my_gizmos
        .arc_3d(
            180.0_f32.to_radians(),
            0.2,
            Isometry3d::new(
                Vec3::ONE,
                Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()),
            ),
            ORANGE,
        )
        .resolution(10);

    // Circles have 32 line-segments by default.
    my_gizmos.circle(Quat::from_rotation_arc(Vec3::Z, Vec3::Y), 3., BLACK);

    // You may want to increase this for larger circles or spheres.
    my_gizmos
        .circle(Quat::from_rotation_arc(Vec3::Z, Vec3::Y), 3.1, NAVY)
        .resolution(64);
    my_gizmos
        .sphere(Isometry3d::IDENTITY, 3.2, BLACK)
        .resolution(64);

    gizmos.arrow(Vec3::ZERO, Vec3::splat(1.5), YELLOW);

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
    if keyboard.just_pressed(KeyCode::KeyT) {
        for (_, config, _) in config_store.iter_mut() {
            config.depth_bias = if config.depth_bias == 0. { -1. } else { 0. };
        }
    }
    if keyboard.just_pressed(KeyCode::KeyP) {
        for (_, config, _) in config_store.iter_mut() {
            // Toggle line perspective
            config.line.perspective ^= true;
            // Increase the line width when line perspective is on
            config.line.width *= if config.line.perspective { 5. } else { 1. / 5. };
        }
    }

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    if keyboard.pressed(KeyCode::ArrowRight) {
        config.line.width += 5. * time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.line.width -= 5. * time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyU) {
        config.line.style = match config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyJ) {
        config.line.joints = match config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    let (my_config, _) = config_store.config_mut::<MyRoundGizmos>();
    if keyboard.pressed(KeyCode::ArrowUp) {
        my_config.line.width += 5. * time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        my_config.line.width -= 5. * time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        my_config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        my_config.line.style = match my_config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyK) {
        my_config.line.joints = match my_config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    if keyboard.just_pressed(KeyCode::KeyB) {
        // AABB gizmos are normally only drawn on entities with a ShowAabbGizmo component
        // We can change this behavior in the configuration of AabbGizmoGroup
        config_store.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    }
}
