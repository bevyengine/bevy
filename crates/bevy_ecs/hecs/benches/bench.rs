// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// modified by Bevy contributors

use bencher::{benchmark_group, benchmark_main, Bencher};
use bevy_hecs::*;

struct Position(f32);
struct Velocity(f32);

fn spawn_tuple(b: &mut Bencher) {
    let mut world = World::new();
    b.iter(|| {
        world.spawn((Position(0.0), Velocity(0.0)));
    });
}

fn spawn_static(b: &mut Bencher) {
    #[derive(Bundle)]
    struct Bundle {
        pos: Position,
        vel: Velocity,
    };

    let mut world = World::new();
    b.iter(|| {
        world.spawn(Bundle {
            pos: Position(0.0),
            vel: Velocity(0.0),
        });
    });
}

fn spawn_batch(b: &mut Bencher) {
    #[derive(Bundle)]
    struct Bundle {
        pos: Position,
        vel: Velocity,
    };

    let mut world = World::new();
    b.iter(|| {
        world
            .spawn_batch((0..1_000).map(|_| Bundle {
                pos: Position(0.0),
                vel: Velocity(0.0),
            }))
            .for_each(|_| {});
        world.clear();
    });
}

fn iterate_100k(b: &mut Bencher) {
    let mut world = World::new();
    for i in 0..100_000 {
        world.spawn((Position(-(i as f32)), Velocity(i as f32)));
    }
    b.iter(|| {
        for (mut pos, vel) in &mut world.query_mut::<(&mut Position, &Velocity)>() {
            pos.0 += vel.0;
        }
    })
}

fn build(b: &mut Bencher) {
    let mut world = World::new();
    let mut builder = EntityBuilder::new();
    b.iter(|| {
        builder.add(Position(0.0)).add(Velocity(0.0));
        world.spawn(builder.build());
    });
}

benchmark_group!(
    benches,
    spawn_tuple,
    spawn_static,
    spawn_batch,
    iterate_100k,
    build
);
benchmark_main!(benches);
