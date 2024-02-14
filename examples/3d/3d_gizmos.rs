//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use std::f32::consts::PI;

use bevy::math::primitives::{
    Capsule3d, Cone, ConicalFrustum, Cuboid, Cylinder, Line3d, Plane3d, Segment3d, Sphere, Torus,
};
use bevy::prelude::*;

fn main() {
    App::new()
        .insert_state(PrimitiveState::Nothing)
        .init_resource::<PrimitiveSegments>()
        .add_plugins(DefaultPlugins)
        .init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, (draw_example_collection, update_config))
        .add_systems(Update, (draw_primitives, update_primitives))
        .run();
}

// We can create our own gizmo config group!
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyRoundGizmos {}

#[derive(Debug, Clone, Resource)]
struct PrimitiveSegments(usize);
impl Default for PrimitiveSegments {
    fn default() -> Self {
        Self(10)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
enum PrimitiveState {
    Nothing,
    Sphere,
    Plane,
    Line,
    LineSegment,
    Cuboid,
    Cylinder,
    Capsule,
    Cone,
    ConicalFrustum,
    Torus,
}

impl PrimitiveState {
    const ALL: [Self; 11] = [
        Self::Sphere,
        Self::Plane,
        Self::Line,
        Self::LineSegment,
        Self::Cuboid,
        Self::Cylinder,
        Self::Capsule,
        Self::Cone,
        Self::ConicalFrustum,
        Self::Torus,
        Self::Nothing,
    ];
    fn next(self) -> Self {
        Self::ALL
            .into_iter()
            .cycle()
            .skip_while(|&x| x != self)
            .nth(1)
            .unwrap()
    }
    fn last(self) -> Self {
        Self::ALL
            .into_iter()
            .rev()
            .cycle()
            .skip_while(|&x| x != self)
            .nth(1)
            .unwrap()
    }
}

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
        material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 250000.0,
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
            Press 'K' or 'J' to cycle through primitives rendered with gizmos\n\
            Press 'H' or 'L' to decrease/increase the amount of segments in the primitives",
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

fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let mut transform = query.single_mut();

    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds() / 2.));
}

fn draw_example_collection(
    mut gizmos: Gizmos,
    mut my_gizmos: Gizmos<MyRoundGizmos>,
    time: Res<Time>,
) {
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

    my_gizmos
        .arc_3d(
            180.0_f32.to_radians(),
            0.2,
            Vec3::ONE,
            Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()),
            Color::ORANGE,
        )
        .segments(10);

    // Circles have 32 line-segments by default.
    my_gizmos.circle(Vec3::ZERO, Direction3d::Y, 3., Color::BLACK);
    // You may want to increase this for larger circles or spheres.
    my_gizmos
        .circle(Vec3::ZERO, Direction3d::Y, 3.1, Color::NAVY)
        .segments(64);
    my_gizmos
        .sphere(Vec3::ZERO, Quat::IDENTITY, 3.2, Color::BLACK)
        .circle_segments(64);

    gizmos.arrow(Vec3::ZERO, Vec3::ONE * 1.5, Color::YELLOW);
}

fn draw_primitives(
    mut gizmos: Gizmos,
    time: Res<Time>,
    primitive_state: Res<State<PrimitiveState>>,
    segments: Res<PrimitiveSegments>,
) {
    let normal = Vec3::new(
        time.elapsed_seconds().sin(),
        time.elapsed_seconds().cos(),
        time.elapsed_seconds().sin().cos(),
    )
    .try_normalize()
    .unwrap_or(Vec3::X);
    let angle = time.elapsed_seconds().to_radians() * 10.0;
    let center = Quat::from_axis_angle(Vec3::Z, angle) * Vec3::X;
    let rotation = Quat::from_rotation_arc(Vec3::Y, normal);
    let segments = segments.0;
    match primitive_state.get() {
        PrimitiveState::Nothing => {}
        PrimitiveState::Sphere => {
            gizmos
                .primitive_3d(Sphere { radius: 1.0 }, center, rotation, Color::default())
                .segments(segments);
        }
        PrimitiveState::Plane => {
            gizmos
                .primitive_3d(
                    Plane3d {
                        normal: Direction3d::Y,
                    },
                    center,
                    rotation,
                    Color::default(),
                )
                .axis_count((segments / 5).max(4))
                .segment_count(segments)
                .segment_length(1.0 / segments as f32);
        }
        PrimitiveState::Line => {
            gizmos.primitive_3d(
                Line3d {
                    direction: Direction3d::X,
                },
                center,
                rotation,
                Color::default(),
            );
        }
        PrimitiveState::LineSegment => {
            gizmos.primitive_3d(
                Segment3d {
                    direction: Direction3d::X,
                    half_length: 1.0,
                },
                center,
                rotation,
                Color::default(),
            );
        }
        PrimitiveState::Cuboid => {
            gizmos.primitive_3d(
                Cuboid {
                    half_size: Vec3::new(1.0, 0.5, 2.0),
                },
                center,
                rotation,
                Color::default(),
            );
        }
        PrimitiveState::Cylinder => {
            gizmos
                .primitive_3d(
                    Cylinder {
                        radius: 1.0,
                        half_height: 1.0,
                    },
                    center,
                    rotation,
                    Color::default(),
                )
                .segments(segments);
        }
        PrimitiveState::Capsule => {
            gizmos
                .primitive_3d(
                    Capsule3d {
                        radius: 1.0,
                        half_length: 1.0,
                    },
                    center,
                    rotation,
                    Color::default(),
                )
                .segments(segments);
        }
        PrimitiveState::Cone => {
            gizmos
                .primitive_3d(
                    Cone {
                        radius: 1.0,
                        height: 1.0,
                    },
                    center,
                    rotation,
                    Color::default(),
                )
                .segments(segments);
        }
        PrimitiveState::ConicalFrustum => {
            gizmos
                .primitive_3d(
                    ConicalFrustum {
                        radius_top: 0.5,
                        radius_bottom: 1.0,
                        height: 1.0,
                    },
                    center,
                    rotation,
                    Color::default(),
                )
                .segments(segments);
        }
        PrimitiveState::Torus => {
            gizmos
                .primitive_3d(
                    Torus {
                        minor_radius: 0.3,
                        major_radius: 1.0,
                    },
                    center,
                    rotation,
                    Color::default(),
                )
                .major_segments(segments)
                .minor_segments((segments / 4).max(1));
        }
    }
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

    if keyboard.just_pressed(KeyCode::KeyA) {
        // AABB gizmos are normally only drawn on entities with a ShowAabbGizmo component
        // We can change this behaviour in the configuration of AabbGizmoGroup
        config_store.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    }
}

fn update_primitives(
    keyboard: Res<ButtonInput<KeyCode>>,
    primitive_state: Res<State<PrimitiveState>>,
    mut next_primitive_state: ResMut<NextState<PrimitiveState>>,
    mut segments: ResMut<PrimitiveSegments>,
    mut segments_f: Local<f32>,
) {
    if keyboard.just_pressed(KeyCode::KeyK) {
        next_primitive_state.set(primitive_state.get().next());
    }
    if keyboard.just_pressed(KeyCode::KeyJ) {
        next_primitive_state.set(primitive_state.get().last());
    }
    if keyboard.pressed(KeyCode::KeyL) {
        *segments_f = (*segments_f + 0.05).max(2.0);
        segments.0 = segments_f.floor() as usize;
    }
    if keyboard.pressed(KeyCode::KeyH) {
        *segments_f = (*segments_f - 0.05).max(2.0);
        segments.0 = segments_f.floor() as usize;
    }
}
