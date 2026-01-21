use bevy_ecs::prelude::*;
use glam::*;

#[derive(Component, Copy, Clone)]
struct Transform(Mat4);

#[derive(Component, Copy, Clone)]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Rotation(Vec3);

#[derive(Component, Copy, Clone)]
struct Velocity(Vec3);

pub struct Benchmark<'w>(World, QueryState<(&'w Velocity, &'w mut Position)>);

impl<'w> Benchmark<'w> {
    pub fn supported() -> bool {
        is_x86_feature_detected!("avx2")
    }

    pub fn new() -> Option<Self> {
        if !Self::supported() {
            return None;
        }

        let mut world = World::new();

        world.spawn_batch(core::iter::repeat_n(
            (
                Transform(Mat4::from_scale(Vec3::ONE)),
                Position(Vec3::X),
                Rotation(Vec3::X),
                Velocity(Vec3::X),
            ),
            10_000,
        ));

        let query = world.query::<(&Velocity, &mut Position)>();
        Some(Self(world, query))
    }

    #[inline(never)]
    pub fn run(&mut self) {
        /// # Safety
        /// avx2 must be supported
        #[target_feature(enable = "avx2")]
        unsafe fn exec(position: &mut [Position], velocity: &[Velocity]) {
            for i in 0..position.len() {
                position[i].0 += velocity[i].0;
            }
        }

        let iter = self.1.contiguous_iter_mut(&mut self.0).unwrap();
        for (velocity, (position, mut ticks)) in iter {
            // SAFETY: checked in new
            unsafe {
                exec(position, velocity);
            }
            // to match the iter_simple benchmark
            ticks.mark_all_as_updated();
        }
    }
}
