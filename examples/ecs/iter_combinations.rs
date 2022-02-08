use bevy::{core::FixedTimestep, pbr::AmbientLight, prelude::*, render::camera::Camera};
use rand::{thread_rng, Rng};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

const DELTA_TIME: f64 = 0.01;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            brightness: 0.03,
            ..Default::default()
        })
        .add_startup_system(generate_bodies)
        .add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(DELTA_TIME))
                .with_system(interact_bodies)
                .with_system(integrate),
        )
        .add_system(look_at_star)
        .insert_resource(ClearColor(Color::BLACK))
        .run();
}

const GRAVITY_CONSTANT: f32 = 0.001;
const NUM_BODIES: usize = 100;

#[derive(Component, Default)]
struct Mass(f32);
#[derive(Component, Default)]
struct Acceleration(Vec3);
#[derive(Component, Default)]
struct LastPos(Vec3);
#[derive(Component)]
struct Star;

#[derive(Bundle, Default)]
struct BodyBundle {
    #[bundle]
    pbr: PbrBundle,
    mass: Mass,
    last_pos: LastPos,
    acceleration: Acceleration,
}

fn generate_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Mesh::from(shape::Icosphere {
        radius: 1.0,
        subdivisions: 3,
    }));

    let color_range = 0.5..1.0;
    let vel_range = -0.5..0.5;

    let mut rng = thread_rng();
    for _ in 0..NUM_BODIES {
        let radius: f32 = rng.gen_range(0.1..0.7);
        let mass_value = radius.powi(3) * 10.;

        let position = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize()
            * rng.gen_range(0.2f32..1.0).powf(1. / 3.)
            * 15.;

        commands.spawn_bundle(BodyBundle {
            pbr: PbrBundle {
                transform: Transform {
                    translation: position,
                    scale: Vec3::splat(radius),
                    ..Default::default()
                },
                mesh: mesh.clone(),
                material: materials.add(
                    Color::rgb(
                        rng.gen_range(color_range.clone()),
                        rng.gen_range(color_range.clone()),
                        rng.gen_range(color_range.clone()),
                    )
                    .into(),
                ),
                ..Default::default()
            },
            mass: Mass(mass_value),
            acceleration: Acceleration(Vec3::ZERO),
            last_pos: LastPos(
                position
                    - Vec3::new(
                        rng.gen_range(vel_range.clone()),
                        rng.gen_range(vel_range.clone()),
                        rng.gen_range(vel_range.clone()),
                    ) * DELTA_TIME as f32,
            ),
        });
    }

    // add bigger "star" body in the center
    let star_radius = 1.;
    commands
        .spawn_bundle(BodyBundle {
            pbr: PbrBundle {
                transform: Transform::from_scale(Vec3::splat(star_radius)),
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    radius: 1.0,
                    subdivisions: 5,
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::ORANGE_RED,
                    emissive: (Color::ORANGE_RED * 2.),
                    ..Default::default()
                }),
                ..Default::default()
            },
            mass: Mass(500.0),
            ..Default::default()
        })
        .insert(Star)
        .with_children(|p| {
            p.spawn_bundle(PointLightBundle {
                point_light: PointLight {
                    color: Color::WHITE,
                    intensity: 400.0,
                    range: 100.0,
                    radius: star_radius,
                    ..Default::default()
                },
                ..Default::default()
            });
        });
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 10.5, -30.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn interact_bodies(mut query: Query<(&Mass, &GlobalTransform, &mut Acceleration)>) {
    let mut iter = query.iter_combinations_mut();
    while let Some([(Mass(m1), transform1, mut acc1), (Mass(m2), transform2, mut acc2)]) =
        iter.fetch_next()
    {
        let delta = transform2.translation - transform1.translation;
        let distance_sq: f32 = delta.length_squared();

        let f = GRAVITY_CONSTANT / distance_sq;
        let force_unit_mass = delta * f;
        acc1.0 += force_unit_mass * *m2;
        acc2.0 -= force_unit_mass * *m1;
    }
}

fn integrate(mut query: Query<(&mut Acceleration, &mut Transform, &mut LastPos)>) {
    let dt_sq = (DELTA_TIME * DELTA_TIME) as f32;
    for (mut acceleration, mut transform, mut last_pos) in query.iter_mut() {
        // verlet integration
        // x(t+dt) = 2x(t) - x(t-dt) + a(t)dt^2 + O(dt^4)

        let new_pos =
            transform.translation + transform.translation - last_pos.0 + acceleration.0 * dt_sq;
        acceleration.0 = Vec3::ZERO;
        last_pos.0 = transform.translation;
        transform.translation = new_pos;
    }
}

fn look_at_star(
    mut camera: Query<&mut Transform, (With<Camera>, Without<Star>)>,
    star: Query<&Transform, With<Star>>,
) {
    let mut camera = camera.single_mut();
    let star = star.single();
    let new_rotation = camera
        .looking_at(star.translation, Vec3::Y)
        .rotation
        .lerp(camera.rotation, 0.1);
    camera.rotation = new_rotation;
}
