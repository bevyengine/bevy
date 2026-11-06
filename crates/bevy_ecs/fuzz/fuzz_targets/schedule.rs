#![no_main]

use std::sync::Arc;

use arbitrary::Arbitrary;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel, SingleThreadedExecutor};
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
pub enum ScheduleOp {
    AddSystem(u8),
    AddSystemInSet(u8, u8),

    OrderBefore(u8, u8),
    OrderAfter(u8, u8),
    SetOrderBefore(u8, u8),
    SetOrderAfter(u8, u8),
    ChainSystems(u8, u8, u8),

    AddRunCondition(u8, bool),
    AddSetRunCondition(u8, bool),

    AmbiguousWith(u8, u8),

    Build,
    BuildAndRun,
}

#[derive(Debug, Arbitrary)]
struct ScheduleFuzzInput {
    ops: Vec<ScheduleOp>,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FuzzSet {
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
}

const FUZZ_SETS: [FuzzSet; 8] = [
    FuzzSet::S0,
    FuzzSet::S1,
    FuzzSet::S2,
    FuzzSet::S3,
    FuzzSet::S4,
    FuzzSet::S5,
    FuzzSet::S6,
    FuzzSet::S7,
];

fn resolve_set(idx: u8) -> FuzzSet {
    FUZZ_SETS[(idx as usize) % FUZZ_SETS.len()]
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct FuzzSchedule;

#[derive(Resource, Default, Clone)]
struct ExecutionLog {
    order: Arc<std::sync::Mutex<Vec<u8>>>,
}

fn sys_0(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(0);
}
fn sys_1(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(1);
}
fn sys_2(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(2);
}
fn sys_3(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(3);
}
fn sys_4(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(4);
}
fn sys_5(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(5);
}
fn sys_6(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(6);
}
fn sys_7(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(7);
}
fn sys_8(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(8);
}
fn sys_9(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(9);
}
fn sys_10(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(10);
}
fn sys_11(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(11);
}
fn sys_12(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(12);
}
fn sys_13(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(13);
}
fn sys_14(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(14);
}
fn sys_15(log: Res<ExecutionLog>) {
    log.order.lock().unwrap().push(15);
}

type SystemFn = fn(Res<ExecutionLog>);

const SYSTEMS: [SystemFn; 16] = [
    sys_0, sys_1, sys_2, sys_3, sys_4, sys_5, sys_6, sys_7, sys_8, sys_9, sys_10, sys_11, sys_12,
    sys_13, sys_14, sys_15,
];

fn resolve_system(idx: u8) -> (u8, SystemFn) {
    let i = (idx as usize) % SYSTEMS.len();
    (i as u8, SYSTEMS[i])
}

fuzz_target!(|input: ScheduleFuzzInput| {
    if input.ops.len() > 128 {
        return;
    }

    let mut world = World::new();
    let log = ExecutionLog::default();
    world.insert_resource(log.clone());

    let mut schedule = Schedule::new(FuzzSchedule);
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.set_build_settings(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Ignore,
        hierarchy_detection: LogLevel::Warn,
        ..Default::default()
    });

    let mut added_systems: [bool; 16] = [false; 16];
    let mut orderings: Vec<(u8, u8)> = Vec::new();

    for op in &input.ops {
        match op {
            ScheduleOp::AddSystem(idx) => {
                let (i, sys) = resolve_system(*idx);
                if !added_systems[i as usize] {
                    added_systems[i as usize] = true;
                    schedule.add_systems(sys);
                }
            }

            ScheduleOp::AddSystemInSet(sys_idx, set_idx) => {
                let (i, sys) = resolve_system(*sys_idx);
                let set = resolve_set(*set_idx);
                if !added_systems[i as usize] {
                    added_systems[i as usize] = true;
                    schedule.add_systems(sys.in_set(set));
                }
            }

            ScheduleOp::OrderBefore(a_idx, b_idx) => {
                let (a, sys_a) = resolve_system(*a_idx);
                let (b, sys_b) = resolve_system(*b_idx);
                if a != b && !added_systems[a as usize] && !added_systems[b as usize] {
                    added_systems[a as usize] = true;
                    added_systems[b as usize] = true;
                    schedule.add_systems((sys_a.before(sys_b), sys_b));
                    orderings.push((a, b));
                }
            }

            ScheduleOp::OrderAfter(a_idx, b_idx) => {
                let (a, sys_a) = resolve_system(*a_idx);
                let (b, sys_b) = resolve_system(*b_idx);
                if a != b && !added_systems[a as usize] && !added_systems[b as usize] {
                    added_systems[a as usize] = true;
                    added_systems[b as usize] = true;
                    schedule.add_systems((sys_a.after(sys_b), sys_b));
                    orderings.push((b, a));
                }
            }

            ScheduleOp::SetOrderBefore(a_idx, b_idx) => {
                let set_a = resolve_set(*a_idx);
                let set_b = resolve_set(*b_idx);
                if set_a != set_b {
                    schedule.configure_sets(set_a.before(set_b));
                }
            }

            ScheduleOp::SetOrderAfter(a_idx, b_idx) => {
                let set_a = resolve_set(*a_idx);
                let set_b = resolve_set(*b_idx);
                if set_a != set_b {
                    schedule.configure_sets(set_a.after(set_b));
                }
            }

            ScheduleOp::ChainSystems(a_idx, b_idx, c_idx) => {
                let (a, sys_a) = resolve_system(*a_idx);
                let (b, sys_b) = resolve_system(*b_idx);
                let (c, sys_c) = resolve_system(*c_idx);
                if a != b
                    && b != c
                    && a != c
                    && !added_systems[a as usize]
                    && !added_systems[b as usize]
                    && !added_systems[c as usize]
                {
                    added_systems[a as usize] = true;
                    added_systems[b as usize] = true;
                    added_systems[c as usize] = true;
                    schedule.add_systems((sys_a, sys_b, sys_c).chain());
                    orderings.push((a, b));
                    orderings.push((b, c));
                }
            }

            ScheduleOp::AddRunCondition(sys_idx, val) => {
                let (i, sys) = resolve_system(*sys_idx);
                let v = *val;
                if !added_systems[i as usize] {
                    added_systems[i as usize] = true;
                    schedule.add_systems(sys.run_if(move || v));
                }
            }

            ScheduleOp::AddSetRunCondition(set_idx, val) => {
                let set = resolve_set(*set_idx);
                let v = *val;
                schedule.configure_sets(set.run_if(move || v));
            }

            ScheduleOp::AmbiguousWith(a_idx, b_idx) => {
                let (a, sys) = resolve_system(*a_idx);
                let set = resolve_set(*b_idx);
                if !added_systems[a as usize] {
                    added_systems[a as usize] = true;
                    schedule.add_systems(sys.ambiguous_with(set));
                }
            }

            ScheduleOp::Build => {
                let _ = schedule.initialize(&mut world);
            }

            ScheduleOp::BuildAndRun => {
                match schedule.initialize(&mut world) {
                    Ok(_) => {
                        log.order.lock().unwrap().clear();
                        schedule.run(&mut world);

                        let execution = log.order.lock().unwrap().clone();
                        for &(before, after) in &orderings {
                            let pos_before = execution.iter().position(|&x| x == before);
                            let pos_after = execution.iter().position(|&x| x == after);
                            if let (Some(pb), Some(pa)) = (pos_before, pos_after) {
                                assert!(
                                    pb < pa,
                                    "Ordering violation: system {} ran at position {} \
                                     but should run before system {} at position {}",
                                    before,
                                    pb,
                                    after,
                                    pa,
                                );
                            }
                        }

                        let mut seen = [false; 16];
                        for &idx in &execution {
                            assert!(!seen[idx as usize], "System {} executed twice", idx,);
                            seen[idx as usize] = true;
                        }
                    }
                    Err(_) => {
                        // Ignore errors, some are expected with random constraints
                    }
                }
            }
        }
    }
});
