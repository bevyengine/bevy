use bevy_ecs::prelude::*;
use core::arch::x86_64::*;
use glam::*;
use rand::prelude::*;

use criterion::BenchmarkId;
use criterion::Criterion;

#[derive(Component, Copy, Clone, Default)]
struct Position(Vec3);

#[derive(Component, Copy, Clone, Default)]
#[repr(transparent)]
struct Health(f32);

//A hyperplane describing solid geometry, (x,y,z) = n with d such that nx + d = 0
#[derive(Component, Copy, Clone, Default)]
struct Wall(Vec3, f32);

struct Benchmark(World);

fn rnd_vec3(rng: &mut ThreadRng) -> Vec3 {
    let x1 = rng.gen_range(-16.0..=16.0);
    let x2 = rng.gen_range(-16.0..=16.0);
    let x3 = rng.gen_range(-16.0..=16.0);

    Vec3::new(x1, x2, x3)
}

fn rnd_wall(rng: &mut ThreadRng) -> Wall {
    let d = rng.gen_range(-16.0..=16.0);

    Wall(rnd_vec3(rng).normalize_or_zero(), d)
}

// AoS to SoA data layout conversion for x86 AVX.
// This code has been adapted from:
//   https://www.intel.com/content/dam/develop/external/us/en/documents/normvec-181650.pdf
#[inline(always)]
// This example is written in a way that benefits from inlined data layout conversion.
fn aos_to_soa_83(aos_inner: &[Vec3; 8]) -> [__m256; 3] {
    unsafe {
        //# SAFETY: Vec3 is repr(C) for x86_64
        let mx0 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(0));
        let mx1 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(4));
        let mx2 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(8));
        let mx3 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(12));
        let mx4 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(16));
        let mx5 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(20));

        let mut m03 = _mm256_castps128_ps256(mx0); // load lower halves
        let mut m14 = _mm256_castps128_ps256(mx1);
        let mut m25 = _mm256_castps128_ps256(mx2);
        m03 = _mm256_insertf128_ps(m03, mx3, 1); // load upper halves
        m14 = _mm256_insertf128_ps(m14, mx4, 1);
        m25 = _mm256_insertf128_ps(m25, mx5, 1);

        let xy = _mm256_shuffle_ps::<0b10011110>(m14, m25); // upper x's and y's
        let yz = _mm256_shuffle_ps::<0b01001001>(m03, m14); // lower y's and z's
        let x = _mm256_shuffle_ps::<0b10001100>(m03, xy);
        let y = _mm256_shuffle_ps::<0b11011000>(yz, xy);
        let z = _mm256_shuffle_ps::<0b11001101>(yz, m25);
        [x, y, z]
    }
}

impl Benchmark {
    fn new(size: i32) -> Benchmark {
        let mut world = World::new();

        let mut rng = rand::thread_rng();

        world.spawn_batch((0..size).map(|_| (Position(rnd_vec3(&mut rng)), Health(100.0))));
        world.spawn_batch((0..(2_i32.pow(12) - 1)).map(|_| (rnd_wall(&mut rng))));

        Self(world)
    }

    fn scalar(mut pos_healths: Query<(&Position, &mut Health)>, walls: Query<&Wall>) {
        pos_healths.for_each_mut(|(position, mut health)| {
            // This forms the scalar path: it behaves just like `for_each_mut`.

            // Optional: disable change detection for more performance.
            let health = &mut health.bypass_change_detection().0;

            // Test each (Position,Health) against each Wall.
            walls.for_each(|wall| {
                let plane = wall.0;

                // Test which side of the wall we are on
                let dotproj = plane.dot(position.0);

                // Test against the Wall's displacement/discriminant value
                if dotproj < wall.1 {
                    //Ouch! Take damage!
                    *health -= 1.0;
                }
            });
        });
    }

    // Perform collision detection against a set of Walls, forming a convex polygon.
    // Each entity has a Position and some Health (initialized to 100.0).
    // If the position of an entity is found to be outside of a Wall, decrement its "health" by 1.0.
    // The effect is cumulative based on the number of walls.
    // An entity entirely inside the convex polygon will have its health remain unchanged.
    fn batched_avx(mut pos_healths: Query<(&Position, &mut Health)>, walls: Query<&Wall>) {
        // Conceptually, this system is executed using two loops: the outer "batched" loop receiving
        // batches of 8 Positions and Health components at a time, and the inner loop iterating over
        // the Walls.

        // There's more than one way to vectorize this system -- this example may not be optimal.
        pos_healths.for_each_mut_batched::<8>(
            |(position, mut health)| {
                // This forms the scalar path: it behaves just like `for_each_mut`.

                // Optional: disable change detection for more performance.
                let health = &mut health.bypass_change_detection().0;

                // Test each (Position,Health) against each Wall.
                walls.for_each(|wall| {
                    let plane = wall.0;

                    // Test which side of the wall we are on
                    let dotproj = plane.dot(position.0);

                    // Test against the Wall's displacement/discriminant value
                    if dotproj < wall.1 {
                        //Ouch! Take damage!
                        *health -= 1.0;
                    }
                });
            },
            |(positions, mut healths)| {
                // This forms the vector path: the closure receives a batch of
                // 8 Positions and 8 Healths as arrays.

                // Optional: disable change detection for more performance.
                let healths = healths.bypass_change_detection();

                // Treat the Health batch as a batch of 8 f32s.
                unsafe {
                    // # SAFETY: Health is repr(transprent)!
                    let healths_raw = healths as *mut Health as *mut f32;
                    let mut healths = _mm256_loadu_ps(healths_raw);

                    // NOTE: array::map optimizes poorly -- it is recommended to unpack your arrays
                    // manually as shown to avoid spurious copies which will impact your performance.
                    let [p0, p1, p2, p3, p4, p5, p6, p7] = positions;

                    // Perform data layout conversion from AoS to SoA.
                    // ps_x will receive all of the X components of the positions,
                    // ps_y will receive all of the Y components
                    // and ps_z will receive all of the Z's.
                    let [ps_x, ps_y, ps_z] =
                        aos_to_soa_83(&[p0.0, p1.0, p2.0, p3.0, p4.0, p5.0, p6.0, p7.0]);

                    // Iterate over each wall without batching.
                    walls.for_each(|wall| {
                        // Test each wall against all 8 positions at once.  The "broadcast" intrinsic
                        // helps us achieve this by duplicating the Wall's X coordinate over an entire
                        // vector register, e.g., [X X ... X]. The same goes for the Wall's Y and Z
                        // coordinates.

                        // This is the exact same formula as implemented in the scalar path, but
                        // modified to be calculated in parallel across each lane.

                        // Multiply all of the X coordinates of each Position against Wall's Normal X
                        let xs_dot = _mm256_mul_ps(ps_x, _mm256_broadcast_ss(&wall.0.x));
                        // Multiply all of the Y coordinates of each Position against Wall's Normal Y
                        let ys_dot = _mm256_mul_ps(ps_y, _mm256_broadcast_ss(&wall.0.y));
                        // Multiply all of the Z coordinates of each Position against Wall's Normal Z
                        let zs_dot = _mm256_mul_ps(ps_z, _mm256_broadcast_ss(&wall.0.z));

                        // Now add them together: the result is a vector register containing the dot
                        // product of each Position against the Wall's Normal vector.
                        let dotprojs = _mm256_add_ps(_mm256_add_ps(xs_dot, ys_dot), zs_dot);

                        // Take the Wall's discriminant/displacement value and broadcast it like before.
                        let wall_d = _mm256_broadcast_ss(&wall.1);

                        // Compare each dot product against the Wall's discriminant, using the
                        // "Less Than" relation as we did in the scalar code.
                        // The result will be be either -1 or zero *as an integer*.
                        let cmp = _mm256_cmp_ps::<_CMP_LT_OS>(dotprojs, wall_d);

                        // Convert the integer values back to f32 values (-1.0 or 0.0).
                        // These form the damage values for each entity.
                        let damages = _mm256_cvtepi32_ps(_mm256_castps_si256(cmp)); //-1.0 or 0.0

                        // Update the healths of each entity being processed with the results of the
                        // collision detection.
                        healths = _mm256_add_ps(healths, damages);
                    });
                    // Now that all Walls have been processed, write the final updated Health values
                    // for this batch of entities back to main memory.
                    _mm256_storeu_ps(healths_raw, healths);
                }
            },
        );
    }
}

pub fn batched_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("batched_compute");
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(9));

    for exp in 14..17 {
        let size = 2_i32.pow(exp) - 1; //Ensure scalar path gets run too (incomplete batch at end)

        group.bench_with_input(
            BenchmarkId::new("autovectorized", size),
            &size,
            |b, &size| {
                let Benchmark(mut world) = Benchmark::new(size);

                let mut system = IntoSystem::into_system(Benchmark::scalar);
                system.initialize(&mut world);
                system.update_archetype_component_access(&world);

                b.iter(move || system.run((), &mut world));
            },
        );

        group.bench_with_input(BenchmarkId::new("batched_avx", size), &size, |b, &size| {
            let Benchmark(mut world) = Benchmark::new(size);

            let mut system = IntoSystem::into_system(Benchmark::batched_avx);
            system.initialize(&mut world);
            system.update_archetype_component_access(&world);

            b.iter(move || system.run((), &mut world));
        });
    }

    group.finish();
}
