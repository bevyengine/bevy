use bevy::{core::FixedTimestep, prelude::*};
use rand::{thread_rng, Rng};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

const DELTA_TIME: f64 = 0.01;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(generate_bodies.system())
        .add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(DELTA_TIME))
                .with_system(interact_bodies.system())
                .with_system(integrate.system()),
        )
        .run();
}

const GRAVITY_CONSTANT: f32 = 0.001;
const SOFTENING: f32 = 0.01;
const NUM_BODIES: usize = 100;

#[derive(Default)]
struct Mass(f32);
#[derive(Default)]
struct Acceleration(Vec3);
#[derive(Default)]
struct LastPos(Vec3);

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

    let pos_range = 1.0..15.0;
    let color_range = 0.5..1.0;
    let vel_range = -0.5..0.5;

    let mut rng = thread_rng();
    for _ in 0..NUM_BODIES {
        let mass_value_cube_root: f32 = rng.gen_range(0.5..4.0);
        let mass_value: f32 = mass_value_cube_root * mass_value_cube_root * mass_value_cube_root;

        let position = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize()
            * rng.gen_range(pos_range.clone());

        commands.spawn_bundle(BodyBundle {
            pbr: PbrBundle {
                transform: Transform {
                    translation: position,
                    scale: Vec3::splat(mass_value_cube_root * 0.1),
                    ..Default::default()
                },
                mesh: mesh.clone(),
                material: materials.add(
                    Color::rgb_linear(
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
    commands
        .spawn_bundle(BodyBundle {
            pbr: PbrBundle {
                transform: Transform {
                    scale: Vec3::splat(0.5),
                    ..Default::default()
                },
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    radius: 1.0,
                    subdivisions: 5,
                })),
                material: materials.add((Color::ORANGE_RED * 10.0).into()),
                ..Default::default()
            },
            mass: Mass(1000.0),
            ..Default::default()
        })
        .insert(PointLight {
            color: Color::ORANGE_RED,
            ..Default::default()
        });
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 10.5, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
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

        let f = GRAVITY_CONSTANT / (distance_sq * (distance_sq + SOFTENING).sqrt());
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
