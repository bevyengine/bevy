use crate::Particles;
use bevy_core::Time;
use bevy_ecs::prelude::*;
use bevy_math::*;
use bevy_tasks::ComputeTaskPool;

pub trait ParticleModifier: Component {
    fn apply(&self, particles: &mut Particles, delta_time: f32);
}

#[derive(Component, Debug, Clone)]
pub struct ConstantForce {
    pub acceleration_per_second: Vec3,
}

impl ParticleModifier for ConstantForce {
    fn apply(&self, particles: &mut Particles, delta_time: f32) {
        let delta_velocity = Vec4::from((self.acceleration_per_second, 0.0)) * delta_time;
        for velocity in particles.velocities.iter_mut() {
            *velocity += delta_velocity;
        }
    }
}

pub fn apply_particle_modifier<T: ParticleModifier>(
    compute_task_pool: Res<ComputeTaskPool>,
    time: Res<Time>,
    mut particles: Query<(&T, &mut Particles)>,
) {
    let delta_time = time.delta_seconds_f64() as f32;
    particles.par_for_each_mut(&compute_task_pool, 8, |(modifier, mut particles)| {
        modifier.apply(&mut particles, delta_time);
    });
}
