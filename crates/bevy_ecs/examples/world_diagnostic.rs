use std::borrow::Cow;

use bevy_ecs::{prelude::*, schedule::NodeId};
use bevy_ecs_macros::{ScheduleLabel, SystemSet};

fn empty_system() {}

fn first_system() {}

fn second_system() {}

#[derive(SystemSet, Hash, Clone, Copy, PartialEq, Eq, Debug)]
enum MySet {
    Set1,
    Set2,
}

#[derive(SystemSet, Hash, Clone, Copy, PartialEq, Eq, Debug)]
#[system_set(base)]
enum MyBaseSet {
    BaseSet1,
    BaseSet2,
}

#[derive(Component)]
struct Counter(usize);

#[derive(Component)]
struct HitPoint(usize);

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScheduleLabel {
    Foo,
    Bar,
}

/// A special label for diagnostic.
/// If the diagnostic system running in same label as CoreSet,
/// then it is not able to get the [`Schedule`] instance from that label.
/// ([`World::run_schedule_ref`] for reference)
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticLabel;

/// World diagnostic example.
/// can be called directly on World or run as a system.
fn diagnostic_world(world: &World) {
    let bundle_size = world.bundles().len();
    let component_size = world.components().len();
    let archetype_size = world.archetypes().len();
    let entity_size = world.entities().len();

    println!("***************");
    println!("World shape:");
    println!("  component: {bundle_size}");
    println!("  bundle: {component_size}");
    println!("  archetype: {archetype_size}");
    println!("  entity: {entity_size}");

    let schedules = world.get_resource::<Schedules>().unwrap();
    diagnostic_schedules(schedules);

    println!("World detail:");
    let bundles = world.bundles().iter().collect::<Vec<_>>();
    if bundle_size > 0 {
        println!("  bundles:");
        for bundle in bundles {
            println!("    {:?}: {:?}", bundle.id(), bundle.components());
        }
    }

    if archetype_size > 0 {
        println!("  archetypes:");
        for archetype in world.archetypes().iter() {
            println!(
                "    {:?}: components:{:?} entity count: {}",
                archetype.id(),
                archetype.components().collect::<Vec<_>>(),
                archetype.len()
            );
        }
    }
}

fn diagnostic_schedules(schedules: &Schedules) {
    let label_with_schedules = schedules.iter().collect::<Vec<_>>();
    println!("  schedules: {}", label_with_schedules.len());

    for (label, schedule) in label_with_schedules {
        println!(
            "    label: {:?} kind:{:?}",
            label,
            schedule.get_executor_kind(),
        );

        let schedule_graph = schedule.graph();
        {
            let hierarchy_dag = schedule_graph.hierarchy();
            println!("    hierachy:");
            for (l, r, _) in hierarchy_dag.graph().all_edges() {
                let l_name = name_for_node_id(l, schedule);
                let r_name = name_for_node_id(r, schedule);
                println!("      {l:?}({l_name:?}) -> {r:?}({r_name:?})");
            }
        }

        {
            let dependency_dag = schedule_graph.dependency();
            println!("    dependency:");
            for (l, r, _) in dependency_dag.graph().all_edges() {
                let l_name = name_for_node_id(l, schedule);
                let r_name = name_for_node_id(r, schedule);
                println!("      {l:?}({l_name:?}) -> {r:?}({r_name:?})");
            }
        }
    }
}

fn name_for_node_id(node_id: NodeId, schedule: &Schedule) -> Option<Cow<'static, str>> {
    match node_id {
        bevy_ecs::schedule::NodeId::System(_) => {
            let system = schedule.get_system_at(node_id)?;
            system.name()
        }
        bevy_ecs::schedule::NodeId::Set(_) => {
            let set = schedule.graph().get_set_at(node_id)?;
            format!("{set:?}").into()
        }
    }
    .into()
}

// In this example we add a counter resource and increase it's value in one system,
// while a different system prints the current count to the console.
fn main() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    {
        let mut diagnostic_shedule = Schedule::default();
        diagnostic_shedule.add_system(diagnostic_world);
        world.add_schedule(diagnostic_shedule, DiagnosticLabel);
    }

    let mut schedule = Schedule::default();
    schedule.configure_set(MySet::Set1);
    schedule.configure_set(MySet::Set2);
    schedule.configure_set(MyBaseSet::BaseSet1);
    schedule.configure_set(MyBaseSet::BaseSet2);

    schedule.add_system(
        empty_system
            .in_set(MySet::Set1)
            .in_base_set(MyBaseSet::BaseSet1),
    );
    schedule.add_system(
        first_system
            .before(second_system)
            .in_set(MySet::Set2)
            .in_base_set(MyBaseSet::BaseSet2),
    );
    schedule.add_system(
        second_system
            .in_set(MySet::Set2)
            .in_base_set(MyBaseSet::BaseSet2),
    );
    world.add_schedule(schedule, ScheduleLabel::Bar);

    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    let player = world.spawn(HitPoint(100)).id();
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).insert((Counter(0),));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).despawn();
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);
}
