//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

fn main() {
    App::new()
        .init_state::<PrimitiveState>()
        .add_plugins(DefaultPlugins)
        .init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_example_collection, update_config))
        .add_systems(Update, (draw_primitives, update_primitives))
        .run();
}

// We can create our own gizmo config group!
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyRoundGizmos {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default)]
enum PrimitiveState {
    #[default]
    Nothing,
    Circle,
    Ellipse,
    Capsule,
    Line,
    Plane,
    Segment,
    Triangle,
    Rectangle,
    RegularPolygon,
}

impl PrimitiveState {
    const ALL: [Self; 10] = [
        Self::Nothing,
        Self::Circle,
        Self::Ellipse,
        Self::Capsule,
        Self::Line,
        Self::Plane,
        Self::Segment,
        Self::Triangle,
        Self::Rectangle,
        Self::RegularPolygon,
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    // text
    commands.spawn(TextBundle::from_section(
        "Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
        Hold 'Up' or 'Down' to change the line width of round gizmos\n\
        Press '1' or '2' to toggle the visibility of straight gizmos or round gizmos\n\
        Press 'K' or 'J' to cycle through primitives rendered with gizmos",
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 24.,
            color: Color::WHITE,
        },
    ));
}

fn draw_example_collection(
    mut gizmos: Gizmos,
    mut my_gizmos: Gizmos<MyRoundGizmos>,
    time: Res<Time>,
) {
    let sin = time.elapsed_seconds().sin() * 50.;
    gizmos.line_2d(Vec2::Y * -sin, Vec2::splat(-80.), Color::RED);
    gizmos.ray_2d(Vec2::Y * sin, Vec2::splat(80.), Color::GREEN);

    // Triangle
    gizmos.linestrip_gradient_2d([
        (Vec2::Y * 300., Color::BLUE),
        (Vec2::new(-255., -155.), Color::RED),
        (Vec2::new(255., -155.), Color::GREEN),
        (Vec2::Y * 300., Color::BLUE),
    ]);

    gizmos.rect_2d(
        Vec2::ZERO,
        time.elapsed_seconds() / 3.,
        Vec2::splat(300.),
        Color::BLACK,
    );

    // The circles have 32 line-segments by default.
    my_gizmos.circle_2d(Vec2::ZERO, 120., Color::BLACK);
    my_gizmos.ellipse_2d(
        Vec2::ZERO,
        time.elapsed_seconds() % TAU,
        Vec2::new(100., 200.),
        Color::YELLOW_GREEN,
    );
    // You may want to increase this for larger circles.
    my_gizmos
        .circle_2d(Vec2::ZERO, 300., Color::NAVY)
        .segments(64);

    // Arcs default amount of segments is linearly interpolated between
    // 1 and 32, using the arc length as scalar.
    my_gizmos.arc_2d(Vec2::ZERO, sin / 10., PI / 2., 350., Color::ORANGE_RED);

    gizmos.arrow_2d(
        Vec2::ZERO,
        Vec2::from_angle(sin / -10. + PI / 2.) * 50.,
        Color::YELLOW,
    );
}

fn draw_primitives(
    mut gizmos: Gizmos,
    time: Res<Time>,
    primitive_state: Res<State<PrimitiveState>>,
) {
    let angle = time.elapsed_seconds();
    let rotation = Mat2::from_angle(angle);
    let position = rotation * Vec2::X;
    let color = Color::WHITE;

    const SIZE: f32 = 50.0;
    match primitive_state.get() {
        PrimitiveState::Nothing => {}
        PrimitiveState::Circle => {
            gizmos.primitive_2d(Circle { radius: SIZE }, position, angle, color);
        }
        PrimitiveState::Ellipse => gizmos.primitive_2d(
            Ellipse {
                half_size: Vec2::new(SIZE, SIZE * 0.5),
            },
            position,
            angle,
            color,
        ),
        PrimitiveState::Capsule => gizmos.primitive_2d(
            Capsule2d {
                radius: SIZE * 0.5,
                half_length: SIZE,
            },
            position,
            angle,
            color,
        ),
        PrimitiveState::Line => drop(gizmos.primitive_2d(
            Line2d {
                direction: Direction2d::X,
            },
            position,
            angle,
            color,
        )),
        PrimitiveState::Plane => gizmos.primitive_2d(
            Plane2d {
                normal: Direction2d::Y,
            },
            position,
            angle,
            color,
        ),
        PrimitiveState::Segment => drop(gizmos.primitive_2d(
            Segment2d {
                direction: Direction2d::X,
                half_length: SIZE * 0.5,
            },
            position,
            angle,
            color,
        )),
        PrimitiveState::Triangle => gizmos.primitive_2d(
            Triangle2d {
                vertices: [Vec2::ZERO, Vec2::Y, Vec2::X].map(|p| p * SIZE * 0.5),
            },
            position,
            angle,
            color,
        ),
        PrimitiveState::Rectangle => gizmos.primitive_2d(
            Rectangle {
                half_size: Vec2::splat(SIZE * 0.5),
            },
            position,
            angle,
            color,
        ),
        PrimitiveState::RegularPolygon => gizmos.primitive_2d(
            RegularPolygon {
                circumcircle: Circle { radius: SIZE * 0.5 },
                sides: 5,
            },
            position,
            angle,
            color,
        ),
    }
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
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
}

fn update_primitives(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_primitive_state: ResMut<NextState<PrimitiveState>>,
    primitive_state: Res<State<PrimitiveState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyJ) {
        next_primitive_state.set(primitive_state.get().last());
    }
    if keyboard.just_pressed(KeyCode::KeyK) {
        next_primitive_state.set(primitive_state.get().next());
    }
}
