//! This example demonstrates bounding volume intersections.

use bevy::{color::palettes::css::*, math::bounding::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<Test>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_text, spin, update_volumes, update_test_state),
        )
        .add_systems(
            PostUpdate,
            (
                render_shapes,
                (
                    aabb_intersection_system.run_if(in_state(Test::AabbSweep)),
                    circle_intersection_system.run_if(in_state(Test::CircleSweep)),
                    ray_cast_system.run_if(in_state(Test::RayCast)),
                    aabb_cast_system.run_if(in_state(Test::AabbCast)),
                    bounding_circle_cast_system.run_if(in_state(Test::CircleCast)),
                ),
                render_volumes,
            )
                .chain(),
        )
        .run();
}

#[derive(Component)]
struct Spin;

fn spin(time: Res<Time>, mut query: Query<&mut Transform, With<Spin>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_z(time.delta_seconds() / 5.);
    }
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum Test {
    AabbSweep,
    CircleSweep,
    #[default]
    RayCast,
    AabbCast,
    CircleCast,
}

fn update_test_state(
    keycode: Res<ButtonInput<KeyCode>>,
    cur_state: Res<State<Test>>,
    mut state: ResMut<NextState<Test>>,
) {
    if !keycode.just_pressed(KeyCode::Space) {
        return;
    }

    use Test::*;
    let next = match **cur_state {
        AabbSweep => CircleSweep,
        CircleSweep => RayCast,
        RayCast => AabbCast,
        AabbCast => CircleCast,
        CircleCast => AabbSweep,
    };
    state.set(next);
}

fn update_text(mut text: Query<&mut Text>, cur_state: Res<State<Test>>) {
    if !cur_state.is_changed() {
        return;
    }

    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;
    text.clear();

    text.push_str("Intersection test:\n");
    use Test::*;
    for &test in &[AabbSweep, CircleSweep, RayCast, AabbCast, CircleCast] {
        let s = if **cur_state == test { "*" } else { " " };
        text.push_str(&format!(" {s} {test:?} {s}\n"));
    }
    text.push_str("\nPress space to cycle");
}

#[derive(Component)]
enum Shape {
    Rectangle(Rectangle),
    Circle(Circle),
    Triangle(Triangle2d),
    Line(Segment2d),
    Capsule(Capsule2d),
    Polygon(RegularPolygon),
}

fn render_shapes(mut gizmos: Gizmos, query: Query<(&Shape, &Transform)>) {
    let color = GRAY;
    for (shape, transform) in query.iter() {
        let translation = transform.translation.xy();
        let rotation = transform.rotation.to_euler(EulerRot::YXZ).2;
        match shape {
            Shape::Rectangle(r) => {
                gizmos.primitive_2d(r, translation, rotation, color);
            }
            Shape::Circle(c) => {
                gizmos.primitive_2d(c, translation, rotation, color);
            }
            Shape::Triangle(t) => {
                gizmos.primitive_2d(t, translation, rotation, color);
            }
            Shape::Line(l) => {
                gizmos.primitive_2d(l, translation, rotation, color);
            }
            Shape::Capsule(c) => {
                gizmos.primitive_2d(c, translation, rotation, color);
            }
            Shape::Polygon(p) => {
                gizmos.primitive_2d(p, translation, rotation, color);
            }
        }
    }
}

#[derive(Component)]
enum DesiredVolume {
    Aabb,
    Circle,
}

#[derive(Component, Debug)]
enum CurrentVolume {
    Aabb(Aabb2d),
    Circle(BoundingCircle),
}

fn update_volumes(
    mut commands: Commands,
    query: Query<
        (Entity, &DesiredVolume, &Shape, &Transform),
        Or<(Changed<DesiredVolume>, Changed<Shape>, Changed<Transform>)>,
    >,
) {
    for (entity, desired_volume, shape, transform) in query.iter() {
        let translation = transform.translation.xy();
        let rotation = transform.rotation.to_euler(EulerRot::YXZ).2;
        match desired_volume {
            DesiredVolume::Aabb => {
                let aabb = match shape {
                    Shape::Rectangle(r) => r.aabb_2d(translation, rotation),
                    Shape::Circle(c) => c.aabb_2d(translation, rotation),
                    Shape::Triangle(t) => t.aabb_2d(translation, rotation),
                    Shape::Line(l) => l.aabb_2d(translation, rotation),
                    Shape::Capsule(c) => c.aabb_2d(translation, rotation),
                    Shape::Polygon(p) => p.aabb_2d(translation, rotation),
                };
                commands.entity(entity).insert(CurrentVolume::Aabb(aabb));
            }
            DesiredVolume::Circle => {
                let circle = match shape {
                    Shape::Rectangle(r) => r.bounding_circle(translation, rotation),
                    Shape::Circle(c) => c.bounding_circle(translation, rotation),
                    Shape::Triangle(t) => t.bounding_circle(translation, rotation),
                    Shape::Line(l) => l.bounding_circle(translation, rotation),
                    Shape::Capsule(c) => c.bounding_circle(translation, rotation),
                    Shape::Polygon(p) => p.bounding_circle(translation, rotation),
                };
                commands
                    .entity(entity)
                    .insert(CurrentVolume::Circle(circle));
            }
        }
    }
}

fn render_volumes(mut gizmos: Gizmos, query: Query<(&CurrentVolume, &Intersects)>) {
    for (volume, intersects) in query.iter() {
        let color = if **intersects { AQUA } else { ORANGE_RED };
        match volume {
            CurrentVolume::Aabb(a) => {
                gizmos.rect_2d(a.center(), 0., a.half_size() * 2., color);
            }
            CurrentVolume::Circle(c) => {
                gizmos.circle_2d(c.center(), c.radius(), color);
            }
        }
    }
}

#[derive(Component, Deref, DerefMut, Default)]
struct Intersects(bool);

const OFFSET_X: f32 = 125.;
const OFFSET_Y: f32 = 75.;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(-OFFSET_X, OFFSET_Y, 0.),
            ..default()
        },
        Shape::Circle(Circle::new(45.)),
        DesiredVolume::Aabb,
        Intersects::default(),
    ));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(0., OFFSET_Y, 0.),
            ..default()
        },
        Shape::Rectangle(Rectangle::new(80., 80.)),
        Spin,
        DesiredVolume::Circle,
        Intersects::default(),
    ));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(OFFSET_X, OFFSET_Y, 0.),
            ..default()
        },
        Shape::Triangle(Triangle2d::new(
            Vec2::new(-40., -40.),
            Vec2::new(-20., 40.),
            Vec2::new(40., 50.),
        )),
        Spin,
        DesiredVolume::Aabb,
        Intersects::default(),
    ));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(-OFFSET_X, -OFFSET_Y, 0.),
            ..default()
        },
        Shape::Line(Segment2d::new(Dir2::from_xy(1., 0.3).unwrap(), 90.)),
        Spin,
        DesiredVolume::Circle,
        Intersects::default(),
    ));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(0., -OFFSET_Y, 0.),
            ..default()
        },
        Shape::Capsule(Capsule2d::new(25., 50.)),
        Spin,
        DesiredVolume::Aabb,
        Intersects::default(),
    ));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(OFFSET_X, -OFFSET_Y, 0.),
            ..default()
        },
        Shape::Polygon(RegularPolygon::new(50., 6)),
        Spin,
        DesiredVolume::Circle,
        Intersects::default(),
    ));

    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn draw_filled_circle(gizmos: &mut Gizmos, position: Vec2, color: Srgba) {
    for r in [1., 2., 3.] {
        gizmos.circle_2d(position, r, color);
    }
}

fn draw_ray(gizmos: &mut Gizmos, ray: &RayCast2d) {
    gizmos.line_2d(
        ray.ray.origin,
        ray.ray.origin + *ray.ray.direction * ray.max,
        WHITE,
    );
    draw_filled_circle(gizmos, ray.ray.origin, FUCHSIA);
}

fn get_and_draw_ray(gizmos: &mut Gizmos, time: &Time) -> RayCast2d {
    let ray = Vec2::new(time.elapsed_seconds().cos(), time.elapsed_seconds().sin());
    let dist = 150. + (0.5 * time.elapsed_seconds()).sin().abs() * 500.;

    let aabb_ray = Ray2d {
        origin: ray * 250.,
        direction: Dir2::new_unchecked(-ray),
    };
    let ray_cast = RayCast2d::from_ray(aabb_ray, dist - 20.);

    draw_ray(gizmos, &ray_cast);
    ray_cast
}

fn ray_cast_system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut volumes: Query<(&CurrentVolume, &mut Intersects)>,
) {
    let ray_cast = get_and_draw_ray(&mut gizmos, &time);

    for (volume, mut intersects) in volumes.iter_mut() {
        let toi = match volume {
            CurrentVolume::Aabb(a) => ray_cast.aabb_intersection_at(a),
            CurrentVolume::Circle(c) => ray_cast.circle_intersection_at(c),
        };
        **intersects = toi.is_some();
        if let Some(toi) = toi {
            draw_filled_circle(
                &mut gizmos,
                ray_cast.ray.origin + *ray_cast.ray.direction * toi,
                LIME,
            );
        }
    }
}

fn aabb_cast_system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut volumes: Query<(&CurrentVolume, &mut Intersects)>,
) {
    let ray_cast = get_and_draw_ray(&mut gizmos, &time);
    let aabb_cast = AabbCast2d {
        aabb: Aabb2d::new(Vec2::ZERO, Vec2::splat(15.)),
        ray: ray_cast,
    };

    for (volume, mut intersects) in volumes.iter_mut() {
        let toi = match *volume {
            CurrentVolume::Aabb(a) => aabb_cast.aabb_collision_at(a),
            CurrentVolume::Circle(_) => None,
        };

        **intersects = toi.is_some();
        if let Some(toi) = toi {
            gizmos.rect_2d(
                aabb_cast.ray.ray.origin + *aabb_cast.ray.ray.direction * toi,
                0.,
                aabb_cast.aabb.half_size() * 2.,
                LIME,
            );
        }
    }
}

fn bounding_circle_cast_system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut volumes: Query<(&CurrentVolume, &mut Intersects)>,
) {
    let ray_cast = get_and_draw_ray(&mut gizmos, &time);
    let circle_cast = BoundingCircleCast {
        circle: BoundingCircle::new(Vec2::ZERO, 15.),
        ray: ray_cast,
    };

    for (volume, mut intersects) in volumes.iter_mut() {
        let toi = match *volume {
            CurrentVolume::Aabb(_) => None,
            CurrentVolume::Circle(c) => circle_cast.circle_collision_at(c),
        };

        **intersects = toi.is_some();
        if let Some(toi) = toi {
            gizmos.circle_2d(
                circle_cast.ray.ray.origin + *circle_cast.ray.ray.direction * toi,
                circle_cast.circle.radius(),
                LIME,
            );
        }
    }
}

fn get_intersection_position(time: &Time) -> Vec2 {
    let x = (0.8 * time.elapsed_seconds()).cos() * 250.;
    let y = (0.4 * time.elapsed_seconds()).sin() * 100.;
    Vec2::new(x, y)
}

fn aabb_intersection_system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut volumes: Query<(&CurrentVolume, &mut Intersects)>,
) {
    let center = get_intersection_position(&time);
    let aabb = Aabb2d::new(center, Vec2::splat(50.));
    gizmos.rect_2d(center, 0., aabb.half_size() * 2., YELLOW);

    for (volume, mut intersects) in volumes.iter_mut() {
        let hit = match volume {
            CurrentVolume::Aabb(a) => aabb.intersects(a),
            CurrentVolume::Circle(c) => aabb.intersects(c),
        };

        **intersects = hit;
    }
}

fn circle_intersection_system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut volumes: Query<(&CurrentVolume, &mut Intersects)>,
) {
    let center = get_intersection_position(&time);
    let circle = BoundingCircle::new(center, 50.);
    gizmos.circle_2d(center, circle.radius(), YELLOW);

    for (volume, mut intersects) in volumes.iter_mut() {
        let hit = match volume {
            CurrentVolume::Aabb(a) => circle.intersects(a),
            CurrentVolume::Circle(c) => circle.intersects(c),
        };

        **intersects = hit;
    }
}
