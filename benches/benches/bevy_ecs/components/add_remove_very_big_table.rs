#![expect(
    dead_code,
    reason = "The `Mat4`s in the structs are used to bloat the size of the structs for benchmarking purposes."
)]

use bevy_ecs::prelude::*;
use glam::*;

#[derive(Component, Copy, Clone)]
struct A<const N: usize>(Mat4);
#[derive(Component, Copy, Clone)]
struct B<const N: usize>(Mat4);
#[derive(Component, Copy, Clone)]
struct C<const N: usize>(Mat4);
#[derive(Component, Copy, Clone)]
struct D<const N: usize>(Mat4);
#[derive(Component, Copy, Clone)]
struct E<const N: usize>(Mat4);
#[derive(Component, Copy, Clone)]
struct F<const N: usize>(Mat4);
#[derive(Component, Copy, Clone)]
struct Z<const N: usize>;

pub struct Benchmark(World, Vec<Entity>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::default();
        let mut entities = Vec::with_capacity(10_000);
        for _ in 0..10_000 {
            entities.push(
                world
                    .spawn((
                        (
                            A::<1>(Mat4::from_scale(Vec3::ONE)),
                            B::<1>(Mat4::from_scale(Vec3::ONE)),
                            C::<1>(Mat4::from_scale(Vec3::ONE)),
                            D::<1>(Mat4::from_scale(Vec3::ONE)),
                            E::<1>(Mat4::from_scale(Vec3::ONE)),
                            A::<2>(Mat4::from_scale(Vec3::ONE)),
                            B::<2>(Mat4::from_scale(Vec3::ONE)),
                            C::<2>(Mat4::from_scale(Vec3::ONE)),
                            D::<2>(Mat4::from_scale(Vec3::ONE)),
                            E::<2>(Mat4::from_scale(Vec3::ONE)),
                        ),
                        (
                            A::<3>(Mat4::from_scale(Vec3::ONE)),
                            B::<3>(Mat4::from_scale(Vec3::ONE)),
                            C::<3>(Mat4::from_scale(Vec3::ONE)),
                            D::<3>(Mat4::from_scale(Vec3::ONE)),
                            E::<3>(Mat4::from_scale(Vec3::ONE)),
                            A::<4>(Mat4::from_scale(Vec3::ONE)),
                            B::<4>(Mat4::from_scale(Vec3::ONE)),
                            C::<4>(Mat4::from_scale(Vec3::ONE)),
                            D::<4>(Mat4::from_scale(Vec3::ONE)),
                            E::<4>(Mat4::from_scale(Vec3::ONE)),
                        ),
                        (
                            A::<5>(Mat4::from_scale(Vec3::ONE)),
                            B::<5>(Mat4::from_scale(Vec3::ONE)),
                            C::<5>(Mat4::from_scale(Vec3::ONE)),
                            D::<5>(Mat4::from_scale(Vec3::ONE)),
                            E::<5>(Mat4::from_scale(Vec3::ONE)),
                            A::<6>(Mat4::from_scale(Vec3::ONE)),
                            B::<6>(Mat4::from_scale(Vec3::ONE)),
                            C::<6>(Mat4::from_scale(Vec3::ONE)),
                            D::<6>(Mat4::from_scale(Vec3::ONE)),
                            E::<6>(Mat4::from_scale(Vec3::ONE)),
                        ),
                        (
                            A::<7>(Mat4::from_scale(Vec3::ONE)),
                            B::<7>(Mat4::from_scale(Vec3::ONE)),
                            C::<7>(Mat4::from_scale(Vec3::ONE)),
                            D::<7>(Mat4::from_scale(Vec3::ONE)),
                            E::<7>(Mat4::from_scale(Vec3::ONE)),
                            Z::<1>,
                            Z::<2>,
                            Z::<3>,
                            Z::<4>,
                            Z::<5>,
                            Z::<6>,
                            Z::<7>,
                        ),
                    ))
                    .id(),
            );
        }

        Self(world, entities)
    }

    pub fn run(&mut self) {
        for entity in &self.1 {
            self.0.entity_mut(*entity).insert((
                F::<1>(Mat4::from_scale(Vec3::ONE)),
                F::<2>(Mat4::from_scale(Vec3::ONE)),
                F::<3>(Mat4::from_scale(Vec3::ONE)),
                F::<4>(Mat4::from_scale(Vec3::ONE)),
                F::<5>(Mat4::from_scale(Vec3::ONE)),
                F::<6>(Mat4::from_scale(Vec3::ONE)),
                F::<7>(Mat4::from_scale(Vec3::ONE)),
            ));
        }

        for entity in &self.1 {
            self.0
                .entity_mut(*entity)
                .remove::<(F<1>, F<2>, F<3>, F<4>, F<5>, F<6>, F<7>)>();
            self.0
                .entity_mut(*entity)
                .remove::<(Z<1>, Z<2>, Z<3>, Z<4>, Z<5>, Z<6>, Z<7>)>();
        }
    }
}
