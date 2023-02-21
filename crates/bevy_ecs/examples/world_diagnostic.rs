use std::borrow::Cow;
use std::fmt::Write;

use bevy_ecs::{prelude::*, schedule::NodeId};
use bevy_ecs_macros::{ScheduleLabel, SystemSet};
use bevy_utils::HashMap;

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
/// If the diagnostic system running in a common used label, like the ones
/// defined as `bevy_app::CoreSchedule`, it is not able to get the corresponding
/// [`Schedule`] instance. ([`World::run_schedule_ref`] for reference)
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticLabel;

/// World diagnostic example.
/// can be called directly on World or run as a system.
fn diagnostic_world_system(world: &World) {
    println!("{}", diagnostic_world(world).unwrap());
}

fn diagnostic_world(world: &World) -> Result<String, std::fmt::Error> {
    let mut result = "".to_string();

    let bundle_size = world.bundles().len();
    let component_size = world.components().len();
    let archetype_size = world.archetypes().len();
    let entity_size = world.entities().len();

    writeln!(result, "World shape:")?;
    writeln!(result, "  component: {bundle_size}")?;
    writeln!(result, "  bundle: {component_size}")?;
    writeln!(result, "  archetype: {archetype_size}")?;
    writeln!(result, "  entity: {entity_size}")?;

    if let Some(schedules) = world.get_resource::<Schedules>() {
        result.push_str(&diagnostic_schedules(schedules)?);
    }

    writeln!(result, "World detail:")?;
    let bundles = world.bundles().iter().collect::<Vec<_>>();
    if bundle_size > 0 {
        writeln!(result, "  bundles:")?;
        for bundle in bundles {
            writeln!(result, "    {:?}: {:?}", bundle.id(), bundle.components())?;
        }
    }

    if archetype_size > 0 {
        writeln!(result, "  archetypes:")?;
        for archetype in world.archetypes().iter() {
            writeln!(
                result,
                "    {:?}: components:{:?} entity count: {}",
                archetype.id(),
                archetype.components().collect::<Vec<_>>(),
                archetype.len()
            )?;
        }
    }

    Ok(result)
}

fn diagnostic_schedules(schedules: &Schedules) -> Result<String, std::fmt::Error> {
    let mut result = "".to_string();

    let label_with_schedules = schedules.iter().collect::<Vec<_>>();
    writeln!(result, "  schedules: {}", label_with_schedules.len())?;

    for (label, schedule) in label_with_schedules {
        let mut id_to_names = HashMap::<NodeId, Cow<'static, str>>::new();
        schedule.systems_for_each(|node_id, system| {
            id_to_names.insert(node_id, system.name());
        });
        for (node_id, set, _, _) in schedule.graph().system_sets() {
            id_to_names.insert(node_id, format!("{:?}", set).into());
        }

        writeln!(
            result,
            "    label: {:?} kind:{:?}",
            label,
            schedule.get_executor_kind(),
        )?;

        let schedule_graph = schedule.graph();
        {
            let hierarchy_dag = schedule_graph.hierarchy();
            writeln!(result, "    hierachy:")?;
            for (l, r, _) in hierarchy_dag.graph().all_edges() {
                let l_name = id_to_names.get(&l).unwrap();
                let r_name = id_to_names.get(&r).unwrap();
                writeln!(result, "      {l:?}({l_name:?}) -> {r:?}({r_name:?})")?;
            }
        }

        {
            let dag = schedule_graph.dependency();
            writeln!(result, "    dependency:")?;
            for (l, r, _) in dag.graph().all_edges() {
                let l_name = id_to_names.get(&l).unwrap();
                let r_name = id_to_names.get(&r).unwrap();
                writeln!(result, "      {l:?}({l_name:?}) -> {r:?}({r_name:?})")?;
            }

            writeln!(result, "    topsorted:")?;
            for node_name in dag
                .cached_topsort()
                .iter()
                .map(|node_id| id_to_names.get(node_id).unwrap())
            {
                writeln!(result, "      {node_name}")?;
            }
        }

        {
            let dag = schedule_graph.dependency_flatten();
            writeln!(result, "    dependency flatten:")?;
            for (l, r, _) in dag.graph().all_edges() {
                let l_name = id_to_names.get(&l).unwrap();
                let r_name = id_to_names.get(&r).unwrap();
                writeln!(result, "      {l:?}({l_name:?}) -> {r:?}({r_name:?})")?;
            }

            writeln!(result, "    topsorted:")?;
            for node_name in dag
                .cached_topsort()
                .iter()
                .map(|node_id| id_to_names.get(node_id).unwrap())
            {
                writeln!(result, "      {node_name}")?;
            }
        }
    }

    Ok(result)
}

// In this example we add a counter resource and increase it's value in one system,
// while a different system prints the current count to the console.
fn main() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    {
        let mut diagnostic_shedule = Schedule::default();
        diagnostic_shedule.add_system(diagnostic_world_system);
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
    {
        println!("adding first_system");
        schedule.add_system(
            first_system
                .before(second_system)
                .in_set(MySet::Set2)
                .in_base_set(MyBaseSet::BaseSet2),
        );
    }
    {
        println!("adding second_system");
        schedule.add_system(
            second_system
                .in_set(MySet::Set2)
                .in_base_set(MyBaseSet::BaseSet2),
        );
    }
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
