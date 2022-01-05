use crate::particles::{ParticleParams, Particles};
use bevy_core::Time;
use bevy_ecs::prelude::*;
use bevy_math::*;
use bevy_render::color::Color;
use bevy_tasks::ComputeTaskPool;
use bevy_transform::prelude::*;
use rand::Rng;
use std::{ops::Range, time::Duration};

#[derive(Debug, Clone)]
pub struct EmitterBurst {
    pub count: Range<usize>,
    pub wait: Duration,
}

pub trait EmitterModifier: Send + Sync + 'static {
    fn modify(&mut self, particle: &mut ParticleParams);
}

#[derive(Component)]
pub struct ParticleEmitter {
    next_burst: Duration,
    burst_idx: usize,
    default_params: ParticleParams,
    default_speed: f32,
    bursts: Vec<EmitterBurst>,
    shape: EmitterShape,
    modifiers: Vec<Box<dyn EmitterModifier>>,
}

impl ParticleEmitter {
    pub fn sphere(center: Vec3, radius: f32) -> ParticleEmitterBuilder {
        ParticleEmitterBuilder::new(EmitterShape::Sphere { center, radius })
    }

    pub fn hemisphere(center: Vec3, radius: f32) -> ParticleEmitterBuilder {
        ParticleEmitterBuilder::new(EmitterShape::Hemisphere { center, radius })
    }
}

pub struct ParticleEmitterBuilder {
    default_params: ParticleParams,
    default_speed: f32,
    bursts: Vec<EmitterBurst>,
    shape: EmitterShape,
    modifiers: Vec<Box<dyn EmitterModifier>>,
}

impl ParticleEmitterBuilder {
    fn new(shape: EmitterShape) -> Self {
        Self {
            default_params: ParticleParams {
                size: 1.0,
                color: Color::WHITE,
                lifetime: 5.0,
                ..Default::default()
            },
            default_speed: 0.0,
            bursts: Vec::new(),
            shape,
            modifiers: Vec::new(),
        }
    }

    pub fn add_burst(mut self, burst: EmitterBurst) -> Self {
        self.bursts.push(burst);
        self
    }

    pub fn add_modifier(mut self, modifier: impl EmitterModifier) -> Self {
        self.modifiers.push(Box::new(modifier));
        self
    }

    pub fn with_default_speed(mut self, speed: f32) -> Self {
        self.default_speed = speed;
        self
    }

    pub fn with_default_color(mut self, color: Color) -> Self {
        self.default_params.color = color;
        self
    }

    pub fn with_default_lifetime(mut self, lifetime: f32) -> Self {
        self.default_params.lifetime = lifetime;
        self
    }

    pub fn with_default_size(mut self, size: f32) -> Self {
        self.default_params.size = size;
        self
    }

    pub fn build(self) -> ParticleEmitter {
        ParticleEmitter {
            next_burst: Duration::from_millis(0),
            burst_idx: 0,
            default_params: self.default_params,
            default_speed: self.default_speed,
            bursts: self.bursts,
            shape: self.shape,
            modifiers: self.modifiers,
        }
    }
}

pub enum EmitterShape {
    Sphere { center: Vec3, radius: f32 },
    Hemisphere { center: Vec3, radius: f32 },
}

impl EmitterShape {
    pub fn sample(&self, rng: &mut impl Rng, params: &mut ParticleParams) {
        match self {
            Self::Sphere { radius, center } => Self::sample_sphere(*center, *radius, rng, params),
            Self::Hemisphere { radius, center } => {
                Self::sample_hemisphere(*center, *radius, rng, params)
            }
        }
    }

    fn sample_sphere(center: Vec3, radius: f32, rng: &mut impl Rng, params: &mut ParticleParams) {
        let position = sample_sphere(rng);
        let r = rng.gen_range(0.0..1.0);
        params.position = position * r * radius + center;
        params.velocity = position;
    }

    fn sample_hemisphere(
        center: Vec3,
        radius: f32,
        rng: &mut impl Rng,
        params: &mut ParticleParams,
    ) {
        let mut position = sample_sphere(rng);
        position.y = f32::abs(position.y);
        let r = rng.gen_range(0.0..1.0);
        params.position = position * r * radius + center;
        params.velocity = position;
    }
}

pub fn emit_particles(
    time: Res<Time>,
    compute_task_pool: Res<ComputeTaskPool>,
    mut particles: Query<(&mut ParticleEmitter, &mut Particles, &GlobalTransform)>,
) {
    let delta_time = time.delta();
    particles.par_for_each_mut(
        &compute_task_pool,
        8,
        |(mut emitter, mut particles, transform)| {
            if !particles.state().is_playing() {
                return;
            }

            let mut remaining = delta_time;
            let mut rng = rand::thread_rng();
            let mut total = 0;
            while remaining > emitter.next_burst {
                let EmitterBurst { count, wait } = emitter.bursts[emitter.burst_idx].clone();
                let exact_count = rng.gen_range(count);
                total += exact_count;

                remaining -= emitter.next_burst;

                emitter.next_burst = wait;
                emitter.burst_idx = (emitter.burst_idx + 1) % emitter.bursts.len();
            }

            emitter.next_burst -= remaining;

            if total > 0 {
                let local_to_world = transform.compute_matrix();
                let target_capacity = particles.len() + total;
                particles.reserve(target_capacity);
                for _ in 0..total {
                    let mut params = emitter.default_params.clone();
                    emitter.shape.sample(&mut rng, &mut params);
                    params.velocity *= emitter.default_speed;
                    params.position = local_to_world.transform_point3(params.position);
                    params.velocity = local_to_world.transform_vector3(params.velocity);
                    for modifier in emitter.modifiers.iter_mut() {
                        modifier.modify(&mut params);
                    }
                    particles.spawn(params);
                }
            }
        },
    );
}

/// Select one point at random on the unit sphere.
fn sample_sphere(rng: &mut impl Rng) -> Vec3 {
    const TWO_PI: f32 = std::f32::consts::PI * 2.0;
    let theta = rng.gen_range(0.0..TWO_PI);
    let z = rng.gen_range(-1.0..1.0);
    let x = f32::sqrt(1.0 - z * z) * f32::cos(theta);
    let y = f32::sqrt(1.0 - z * z) * f32::sin(theta);

    Vec3::from((x, y, z))
}
