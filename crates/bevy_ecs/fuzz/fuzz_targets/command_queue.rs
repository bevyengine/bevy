#![no_main]
#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use arbitrary::Arbitrary;
use bevy_ecs::prelude::*;
use bevy_ecs::world::CommandQueue;
use bevy_ecs_fuzz::*;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
pub enum CommandOp {
    PushSmall(u8),
    PushMedium(u32, u32),
    PushLarge(u64, u64, u64, u64),
    PushZst,

    PushSpawnA(CompA),
    PushSpawnB(CompB),

    PushInsertResource(u64),

    PushRecursiveSpawn,

    Apply,

    AppendAndApply,

    DropQueue,
}

#[derive(Debug, Arbitrary)]
struct CommandFuzzInput {
    ops: Vec<CommandOp>,
}

struct DropToken(Arc<AtomicU32>);
impl Drop for DropToken {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

struct SmallCmd(u8, DropToken);
impl Command for SmallCmd {
    type Out = ();
    fn apply(self, _world: &mut World) {}
}

struct MediumCmd(u32, u32, DropToken);
impl Command for MediumCmd {
    type Out = ();
    fn apply(self, _world: &mut World) {}
}

struct LargeCmd(u64, u64, u64, u64, DropToken);
impl Command for LargeCmd {
    type Out = ();
    fn apply(self, _world: &mut World) {}
}

struct ZstCmd(DropToken);
impl Command for ZstCmd {
    type Out = ();
    fn apply(self, _world: &mut World) {}
}

struct SpawnACmd(CompA);
impl Command for SpawnACmd {
    type Out = ();
    fn apply(self, world: &mut World) {
        world.spawn(self.0);
    }
}

struct SpawnBCmd(CompB);
impl Command for SpawnBCmd {
    type Out = ();
    fn apply(self, world: &mut World) {
        world.spawn(self.0);
    }
}

#[derive(Resource)]
struct FuzzResource(u64);

struct InsertResourceCmd(u64);
impl Command for InsertResourceCmd {
    type Out = ();
    fn apply(self, world: &mut World) {
        world.insert_resource(FuzzResource(self.0));
    }
}

struct RecursiveSpawnCmd;
impl Command for RecursiveSpawnCmd {
    type Out = ();
    fn apply(self, world: &mut World) {
        world.commands().queue(|world: &mut World| {
            world.spawn(CompA(999));
        });
        world.flush();
    }
}

fuzz_target!(|input: CommandFuzzInput| {
    if input.ops.len() > 256 {
        return;
    }

    let mut world = World::new();
    let mut queue = CommandQueue::default();

    let drop_count = Arc::new(AtomicU32::new(0));
    let mut total_pushed: u32 = 0;

    let token = || DropToken(drop_count.clone());

    for op in &input.ops {
        match op {
            CommandOp::PushSmall(v) => {
                total_pushed += 1;
                queue.push(SmallCmd(*v, token()));
            }
            CommandOp::PushMedium(a, b) => {
                total_pushed += 1;
                queue.push(MediumCmd(*a, *b, token()));
            }
            CommandOp::PushLarge(a, b, c, d) => {
                total_pushed += 1;
                queue.push(LargeCmd(*a, *b, *c, *d, token()));
            }
            CommandOp::PushZst => {
                total_pushed += 1;
                queue.push(ZstCmd(token()));
            }

            CommandOp::PushSpawnA(a) => {
                queue.push(SpawnACmd(a.clone()));
            }
            CommandOp::PushSpawnB(b) => {
                queue.push(SpawnBCmd(b.clone()));
            }
            CommandOp::PushInsertResource(v) => {
                queue.push(InsertResourceCmd(*v));
            }
            CommandOp::PushRecursiveSpawn => {
                queue.push(RecursiveSpawnCmd);
            }

            CommandOp::Apply => {
                queue.apply(&mut world);
                assert!(queue.is_empty(), "Queue not empty after apply");
            }

            CommandOp::AppendAndApply => {
                let mut secondary = CommandQueue::default();
                total_pushed += 1;
                secondary.push(SmallCmd(0, token()));
                queue.append(&mut secondary);
                queue.apply(&mut world);
                assert!(queue.is_empty(), "Queue not empty after append+apply");
            }

            CommandOp::DropQueue => {
                drop(queue);
                queue = CommandQueue::default();
            }
        }
    }

    drop(queue);

    assert_eq!(
        drop_count.load(Ordering::Relaxed),
        total_pushed,
        "Command consume count mismatch: consumed={}, pushed={}",
        drop_count.load(Ordering::Relaxed),
        total_pushed,
    );
});
