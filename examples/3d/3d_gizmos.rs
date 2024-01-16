//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use std::f32::consts::PI;

use bevy::math::primitives::Direction3d;
use bevy::prelude::*;
use bevy_internal::gizmos::primitives::GizmoPrimitive3d;
use bevy_internal::prelude::primitives::{
    Capsule, Cone, ConicalFrustum, Cuboid, Cylinder, Direction3d, Line3d, Plane3d, Segment3d,
    Sphere, Torus,
};

fn main() {
    App::new()
        .insert_state(PrimitiveState::Nothing)
        .init_resource::<PrimitiveSegments>()
        .add_plugins(DefaultPlugins)
        .init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, (system, rotate_camera, update_config))
        .run();
}

// We can create our own gizmo config group!
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyRoundGizmos {}

#[derive(Debug, Clone, Resource)]
pub struct PrimitiveSegments(usize);
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
    pub fn from_raw(n: u8) -> Self {
        match n {
            1 => Self::Sphere,
            2 => Self::Plane,
            3 => Self::Line,
            4 => Self::LineSegment,
            5 => Self::Cuboid,
            6 => Self::Cylinder,
            7 => Self::Capsule,
            8 => Self::Cone,
            9 => Self::ConicalFrustum,
            10 => Self::Torus,
            _ => Self::Nothing,
        }
    }

    pub fn count() -> u8 {
        11
    }

    pub fn next(self) -> Self {
        let next = (self as u8 + 1) % Self::count();
        Self::from_raw(next)
    }

    pub fn last(self) -> Self {
        let next = (self as u8 + (Self::count() - 1)) % Self::count();
        Self::from_raw(next)
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
        mesh: meshes.add(shape::Plane::from_size(5.0)),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Cube { size: 1.0 }),
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

fn system(
    mut gizmos: Gizmos,
    mut my_gizmos: Gizmos<MyRoundGizmos>,
    time: Res<Time>,
    primitive_state: Res<State<PrimitiveState>>,
    segments: Res<PrimitiveSegments>,
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

    let angle = time.elapsed_seconds().to_radians() * 10.0;
    let center = Quat::from_axis_angle(Vec3::Z, angle) * Vec3::X;
    let normal = (Quat::from_axis_angle(Vec3::ONE, angle) * Vec3::Y).normalize();
    let rotation = Quat::from_rotation_arc(Vec3::Y, normal);
    let segments = segments.0;
    match primitive_state.get() {
        PrimitiveState::Nothing => {}
        PrimitiveState::Sphere => {
            gizmos
                .primitive_3d(Sphere { radius: 1.0 })
                .center(center)
                .rotation(rotation)
                .segments(segments);
        }
        PrimitiveState::Plane => {
            gizmos
                .primitive_3d(Plane3d {
                    normal: Direction3d::new(Vec3::Y).unwrap(),
                })
                .normal_position(center)
                .rotation(rotation);
        }
        PrimitiveState::Line => {
            gizmos
                .primitive_3d(Line3d {
                    direction: Direction3d::new(Vec3::X).unwrap(),
                })
                .start_position(center)
                .rotation(rotation);
        }
        PrimitiveState::LineSegment => {
            gizmos
                .primitive_3d(Segment3d {
                    direction: Direction3d::new(Vec3::X).unwrap(),
                    half_length: 1.0,
                })
                .start_position(center)
                .rotation(rotation);
        }
        PrimitiveState::Cuboid => {
            gizmos
                .primitive_3d(Cuboid {
                    half_size: Vec3::new(1.0, 0.5, 2.0),
                })
                .center(center)
                .rotation(rotation);
        }
        PrimitiveState::Cylinder => {
            gizmos
                .primitive_3d(Cylinder {
                    radius: 1.0,
                    half_height: 1.0,
                })
                .center(center)
                .normal(normal)
                .segments(segments);
        }
        PrimitiveState::Capsule => {
            gizmos
                .primitive_3d(Capsule {
                    radius: 1.0,
                    half_length: 1.0,
                })
                .center(center)
                .normal(normal)
                .segments(segments);
        }
        PrimitiveState::Cone => {
            gizmos
                .primitive_3d(Cone {
                    radius: 1.0,
                    height: 1.0,
                })
                .center(center)
                .normal(normal)
                .segments(segments);
        }
        PrimitiveState::ConicalFrustum => {
            gizmos
                .primitive_3d(ConicalFrustum {
                    radius_top: 0.5,
                    radius_bottom: 1.0,
                    height: 1.0,
                })
                .center(center)
                .normal(normal)
                .segments(segments);
        }
        PrimitiveState::Torus => {
            gizmos
                .primitive_3d(Torus {
                    minor_radius: 0.3,
                    major_radius: 1.0,
                })
                .center(center)
                .normal(normal)
                .major_segments(segments)
                .minor_segments((segments / 4).max(1));
        }
    }
}

fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let mut transform = query.single_mut();

    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds() / 2.));
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    primitive_state: Res<State<PrimitiveState>>,
    mut next_primitive_state: ResMut<NextState<PrimitiveState>>,
    mut segments: ResMut<PrimitiveSegments>,
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
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        next_primitive_state.set(primitive_state.get().next());
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        next_primitive_state.set(primitive_state.get().last());
    }
    if keyboard.just_pressed(KeyCode::KeyM) {
        segments.0 += 1;
    }
    if keyboard.just_pressed(KeyCode::KeyN) {
        segments.0 -= 1;
    }
}
