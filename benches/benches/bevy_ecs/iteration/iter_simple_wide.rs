use bevy_ecs::prelude::*;
use glam::*;

#[derive(Component, Copy, Clone)]
struct Transform(Mat4);

#[derive(Component, Copy, Clone)]
struct Position<const X: usize>(Vec3);

#[derive(Component, Copy, Clone)]
struct Rotation(Vec3);

#[derive(Component, Copy, Clone)]
struct Velocity<const X: usize>(Vec3);

pub struct Benchmark<'w>(
    World,
    QueryState<(
        &'w Velocity<0>,
        &'w mut Position<0>,
        &'w Velocity<1>,
        &'w mut Position<1>,
        &'w Velocity<2>,
        &'w mut Position<2>,
        &'w Velocity<3>,
        &'w mut Position<3>,
        &'w Velocity<4>,
        &'w mut Position<4>,
    )>,
);

impl<'w> Benchmark<'w> {
    pub fn new() -> Self {
        let mut world = World::new();

        // TODO: batch this
        for _ in 0..10_000 {
            world.spawn((
                Transform(Mat4::from_scale(Vec3::ONE)),
                Rotation(Vec3::X),
                Position::<0>(Vec3::X),
                Velocity::<0>(Vec3::X),
                Position::<1>(Vec3::X),
                Velocity::<1>(Vec3::X),
                Position::<2>(Vec3::X),
                Velocity::<2>(Vec3::X),
                Position::<3>(Vec3::X),
                Velocity::<3>(Vec3::X),
                Position::<4>(Vec3::X),
                Velocity::<4>(Vec3::X),
            ));
        }

        let query = world.query();
        Self(world, query)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        for mut item in self.1.iter_mut(&mut self.0) {
            item.1 .0 += item.0 .0;
            item.3 .0 += item.2 .0;
            item.5 .0 += item.4 .0;
            item.7 .0 += item.6 .0;
        }
    }
}
