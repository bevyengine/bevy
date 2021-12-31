use bevy::{particles::modifiers, prelude::*};
use std::time::Duration;

fn create_scene(mut commands: Commands) {
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn create_particles(mut commands: Commands, mut materials: ResMut<Assets<ParticleMaterial>>) {
    let particles = Particles::new(1000);
    commands
        .spawn()
        .insert(particles)
        .insert_bundle(ParticleBundle {
            transform: Transform {
                translation: Vec3::from((0.0, -1.0, 0.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(materials.add(ParticleMaterial {
            base_color_texture: None,
        }))
        .insert(modifiers::ConstantForce {
            acceleration_per_second: Vec3::from((0.0, 5.0, 0.0)),
        })
        .insert(
            ParticleEmitter::hemisphere(Vec3::ZERO, 1.0)
                .add_burst(EmitterBurst {
                    count: 5..10,
                    wait: Duration::from_millis(1),
                })
                .with_default_color(Color::rgba(0.5, 0.5, 0.5, 0.1))
                .with_default_lifetime(1.5)
                .with_default_speed(0.3)
                .with_default_size(0.2)
                .build(),
        );
}

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(create_scene)
        .add_startup_system(create_particles)
        .run()
}
