use bevy_core::Time;
use bevy_ecs::prelude::*;
use bevy_math::*;
use bevy_render::{color::Color, primitives::Aabb};
use bevy_tasks::ComputeTaskPool;
use rand::{rngs::SmallRng, Rng, SeedableRng};

#[derive(Debug, Default, Clone)]
pub struct ParticleParams {
    pub position: Vec3,
    pub rotation: f32,
    pub size: f32,
    pub velocity: Vec3,
    pub angular_velocity: f32,
    pub color: Color,
    pub lifetime: f32,
}

#[derive(Debug, Clone)]
pub struct Particle<'a> {
    pub position: &'a Vec4,
    pub size: &'a f32,
    pub velocity: &'a Vec4,
    pub color: &'a Vec4,
    // pub lifetime: &'a f32,
}

#[derive(Debug)]
pub struct ParticleMut<'a> {
    pub position: &'a mut Vec4,
    pub size: &'a mut f32,
    pub velocity: &'a mut Vec4,
    pub color: &'a mut Vec4,
    // pub lifetime: &'a mut f32,
}

#[derive(Component, Clone)]
/// A container component for a batch of particles.
pub struct Particles {
    pub(crate) lifetime: f32,
    // X, Y, Z - world coordinates
    // W - 1D rotation
    pub(crate) positions: Vec<Vec4>,
    pub(crate) colors: Vec<Vec4>,
    // X, Y, Z - world coordinates
    // W - 1D rotation
    pub(crate) velocities: Vec<Vec4>,
    pub(crate) lerp_factors: Vec<f32>,
    pub(crate) sizes: Vec<f32>,
    pub(crate) starts: Vec<f32>,
    pub(crate) expirations: Vec<f32>,
    // TODO(james7132): make this user initializable.
    rng: SmallRng,
}

impl Default for Particles {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Particles {
    pub fn new(capacity: usize) -> Self {
        Self {
            lifetime: 0.0,
            positions: Vec::with_capacity(capacity),
            colors: Vec::with_capacity(capacity),
            velocities: Vec::with_capacity(capacity),
            sizes: Vec::with_capacity(capacity),
            lerp_factors: Vec::with_capacity(capacity),
            starts: Vec::with_capacity(capacity),
            expirations: Vec::with_capacity(capacity),
            rng: SmallRng::from_entropy(),
        }
    }

    /// Gets a read-only reference to a particle.
    ///
    /// # Panics
    /// Panics if the provided index is out of bounds.
    pub fn get<'a>(&'a self, idx: usize) -> Particle<'a> {
        Particle {
            position: &self.positions[idx],
            velocity: &self.velocities[idx],
            color: &self.colors[idx],
            size: &self.sizes[idx],
            // lifetime: &self.lifetimes[idx],
        }
    }

    /// Gets a mutable reference to a particle.
    ///
    /// # Panics
    /// Panics if the provided index is out of bounds.
    pub fn get_mut<'a>(&'a mut self, idx: usize) -> ParticleMut<'a> {
        ParticleMut {
            position: &mut self.positions[idx],
            size: &mut self.sizes[idx],
            velocity: &mut self.velocities[idx],
            color: &mut self.colors[idx],
        }
    }

    /// Spawns a single particle with the given parameters.
    ///
    /// If spawning multiple at the same time, use `spawn_batch` instead.
    #[inline(always)]
    pub fn spawn(&mut self, params: ParticleParams) {
        self.spawn_batch(&[params]);
    }

    /// Spawns a batch of particles with the given parameters.
    #[inline(always)]
    pub fn spawn_batch(&mut self, batch: &[ParticleParams]) {
        let iterator = batch.into_iter();
        let new_len = self.len() + batch.len();
        self.reserve(new_len);
        unsafe {
            let len = self.len();
            for (idx, param) in iterator.enumerate() {
                self.spawn_unchecked(len + idx, param);
            }
            self.flush(new_len);
        }
    }

    #[inline(always)]
    unsafe fn spawn_unchecked(&mut self, idx: usize, params: &ParticleParams) {
        *self.positions.get_unchecked_mut(idx) = Vec4::from((params.position, params.rotation));
        *self.velocities.get_unchecked_mut(idx) =
            Vec4::from((params.velocity, params.angular_velocity));
        *self.colors.get_unchecked_mut(idx) = params.color.as_rgba_f32().into();
        *self.sizes.get_unchecked_mut(idx) = params.size;
        *self.lerp_factors.get_unchecked_mut(idx) = self.rng.gen_range(0.0..1.0);
        *self.starts.get_unchecked_mut(idx) = self.lifetime;
        *self.expirations.get_unchecked_mut(idx) = self.lifetime + params.lifetime;
    }

    /// Consumes another Particles instance and merges in it's particles.
    pub fn merge(&mut self, batch: impl Into<Particles>) {
        let batch = batch.into();
        self.positions.extend(batch.positions);
        self.velocities.extend(batch.velocities);
        self.colors.extend(batch.colors);
        self.sizes.extend(batch.sizes);
        self.lerp_factors.extend(batch.lerp_factors);
        self.starts.extend(batch.starts);
        self.expirations.extend(batch.expirations);
    }

    pub fn iter<'a>(&'a self) -> ParticleIter<'a> {
        ParticleIter {
            idx: 0,
            particles: self,
        }
    }

    pub fn iter_mut<'a>(&'a mut self) -> ParticleIterMut<'a> {
        ParticleIterMut {
            idx: 0,
            particles: self,
        }
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn capacity(&self) -> usize {
        self.positions.capacity()
    }

    pub fn reserve(&mut self, capacity: usize) {
        self.positions.reserve(capacity);
        self.sizes.reserve(capacity);
        self.lerp_factors.reserve(capacity);
        self.velocities.reserve(capacity);
        self.colors.reserve(capacity);
        self.starts.reserve(capacity);
        self.expirations.reserve(capacity);
    }

    pub fn clear(&mut self) {
        self.lifetime = 0.0;
        self.positions.clear();
        self.sizes.clear();
        self.lerp_factors.clear();
        self.velocities.clear();
        self.colors.clear();
        self.starts.clear();
        self.expirations.clear();
    }

    pub fn compute_aabb(&self) -> Option<Aabb> {
        if self.len() <= 0 {
            return None;
        }

        let mut min = Vec4::splat(f32::MAX);
        let mut max = Vec4::splat(f32::MIN);
        for position in self.positions.iter() {
            min = position.min(min);
            max = position.max(max);
        }
        Some(Aabb::from_min_max(min.xyz(), max.xyz()))
    }

    /// Gets a ratio of how much of a particle's lifetime has passed. Will be 0.0 when the
    /// particle is newly spawned, and 1.0 or greater when the particle is about to be killed.
    ///
    /// # Safety
    /// `idx` must be a particle index, no bounds checking is done here.
    pub unsafe fn lifetime_ratio(&self, idx: usize) -> f32 {
        let start = self.starts.get_unchecked(idx);
        let end = self.expirations.get_unchecked(idx);
        (self.lifetime - start) / (end - start)
    }

    #[inline(always)]
    pub fn advance_particles(&mut self, delta_time: f32) {
        self.lifetime += delta_time;

        if self.len() <= 0 {
            return;
        }

        let delta_time = Vec4::splat(delta_time);
        let mut last = self.len() - 1;
        let mut idx = 0;
        unsafe {
            while idx <= last && last > 0 {
                // SAFE: Both idx and last are always valid indicies
                if *self.expirations.get_unchecked(last) <= self.lifetime {
                    // Avoids the copy in the second block.
                    last -= 1;
                } else if *self.expirations.get_unchecked(idx) <= self.lifetime {
                    self.kill(idx, last);
                    last -= 1;
                } else {
                    let position = self.positions.get_unchecked_mut(idx);
                    let velocity = self.velocities.get_unchecked(idx);
                    *position += *velocity * delta_time;
                    idx += 1;
                }
            }
            // SAFE: the set length is always smaller than the original length or underflowed.
            if last == 0 {
                self.flush(0);
            } else {
                self.flush(last + 1);
            }
        }
    }

    #[inline(always)]
    unsafe fn kill(&mut self, idx: usize, end: usize) {
        debug_assert!(idx <= end);
        *self.positions.get_unchecked_mut(idx) = *self.positions.get_unchecked(end);
        *self.velocities.get_unchecked_mut(idx) = *self.velocities.get_unchecked(end);
        *self.colors.get_unchecked_mut(idx) = *self.colors.get_unchecked(end);
        *self.sizes.get_unchecked_mut(idx) = *self.sizes.get_unchecked(end);
        *self.lerp_factors.get_unchecked_mut(idx) = *self.lerp_factors.get_unchecked(end);
        *self.starts.get_unchecked_mut(idx) = *self.starts.get_unchecked(end);
        *self.expirations.get_unchecked_mut(idx) = *self.expirations.get_unchecked(end);
    }

    #[inline(always)]
    unsafe fn flush(&mut self, len: usize) {
        self.positions.set_len(len);
        self.velocities.set_len(len);
        self.colors.set_len(len);
        self.sizes.set_len(len);
        self.lerp_factors.set_len(len);
        self.starts.set_len(len);
        self.expirations.set_len(len);
    }
}

/// An iterator of read-only particles.
pub struct ParticleIter<'a> {
    idx: usize,
    particles: &'a Particles,
}

impl<'a> Iterator for ParticleIter<'a> {
    type Item = Particle<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.particles.len() {
            None
        } else {
            let particle = self.particles.get(self.idx);
            self.idx += 1;
            Some(particle)
        }
    }
}

/// An iterator of mutable particles.
pub struct ParticleIterMut<'a> {
    idx: usize,
    particles: &'a mut Particles,
}

impl<'a> Iterator for ParticleIterMut<'a> {
    type Item = ParticleMut<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.particles.len() {
            None
        } else {
            unsafe {
                let particles = &mut self.particles;
                let particle = ParticleMut {
                    position: &mut *particles.positions.as_mut_ptr().add(self.idx),
                    size: &mut *particles.sizes.as_mut_ptr().add(self.idx),
                    velocity: &mut *particles.velocities.as_mut_ptr().add(self.idx),
                    color: &mut *particles.colors.as_mut_ptr().add(self.idx),
                };
                self.idx += 1;
                Some(particle)
            }
        }
    }
}

pub fn update_particles(
    time: Res<Time>,
    compute_task_pool: Res<ComputeTaskPool>,
    mut particles: Query<&mut Particles>,
) {
    let delta_time = time.delta_seconds_f64() as f32;
    particles.par_for_each_mut(&compute_task_pool, 8, |mut particles| {
        particles.advance_particles(delta_time);
    });
}
