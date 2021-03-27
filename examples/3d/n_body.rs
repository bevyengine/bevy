use bevy::{core::FixedTimestep, prelude::*};
use rand::{thread_rng, Rng};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(generate_bodies.system())
        .add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(0.01))
                .with_system(interact_bodies.system())
                .with_system(apply_velocity.system()),
        )
        .run();
}

const GRAVITY_CONSTANT: f32 = 0.01;

struct Mass(f32);
struct Velocity(Vec3);

#[derive(Bundle)]
struct BodyBundle {
    #[bundle]
    pbr: PbrBundle,
    mass: Mass,
    velocity: Velocity,
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

    let pos_range = 5.0..20.0;
    let color_range = 0.5..1.0;
    let vel_range = -0.5..0.5;

    let mut rng = thread_rng();
    for _ in 0..200 {
        let mass_value_cube_root: f32 = rng.gen_range(0.9..8.0);
        let mass_value: f32 = mass_value_cube_root * mass_value_cube_root * mass_value_cube_root;

        commands.spawn_bundle(BodyBundle {
            pbr: PbrBundle {
                transform: Transform {
                    translation: Vec3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                    )
                    .normalize()
                        * rng.gen_range(pos_range.clone()),
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
            velocity: Velocity(Vec3::new(
                rng.gen_range(vel_range.clone()),
                rng.gen_range(vel_range.clone()),
                rng.gen_range(vel_range.clone()),
            )),
        });
    }

    // add bigger "star" body in the center
    commands
        .spawn_bundle(BodyBundle {
            pbr: PbrBundle {
                transform: Transform {
                    scale: Vec3::splat(3.0),
                    ..Default::default()
                },
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    radius: 1.0,
                    subdivisions: 5,
                })),
                material: materials.add((Color::ORANGE_RED * 10.0).into()),
                ..Default::default()
            },
            mass: Mass(1500.0),
            velocity: Velocity(Vec3::ZERO),
        })
        .insert(Light {
            color: Color::ORANGE_RED,
            ..Default::default()
        });
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 10.5, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn interact_bodies(mut query: Query<(&Mass, &GlobalTransform, &mut Velocity)>) {
    for [(Mass(m1), transform1, mut vel1), (Mass(m2), transform2, mut vel2)] in query.k_iter_mut() {
        let delta = transform2.translation - transform1.translation;
        let distance_sq: f32 = delta.length_squared();
        let delta_norm = delta / distance_sq.sqrt();
        let force = delta_norm * (GRAVITY_CONSTANT * (m1 + m2) / distance_sq.max(0.01));
        let velocity_change = force;
        vel1.0 += velocity_change / *m1;
        vel2.0 -= velocity_change / *m2;
    }
}

fn apply_velocity(time: Res<Time>, mut query: Query<(&Velocity, &mut Transform)>) {
    for (velocity, mut transform) in query.iter_mut() {
        transform.translation += velocity.0 * time.delta_seconds()
    }
}
