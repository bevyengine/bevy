use std::borrow::Cow;
use std::fmt::Write;

use bevy_ecs::{
    component::ComponentId,
    prelude::*,
    schedule::{Dag, NodeId},
};
use bevy_ecs_macros::{ScheduleLabel, SystemSet};
use bevy_utils::HashMap;

fn empty_system() {}

fn first_system() {}

fn second_system() {}

fn increase_game_state_count(mut state: ResMut<GameState>) {
    state.counter += 1;
}

fn sync_counter(state: Res<GameState>, mut query: Query<&mut Counter>) {
    for mut counter in query.iter_mut() {
        counter.0 = state.counter;
    }
}

#[derive(Resource, Default)]
struct GameState {
    counter: usize,
}

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

#[derive(Component)]
#[component(storage = "SparseSet")]
struct HighlightFlag;

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
    let resource_size = world.storages().resources.len();
    let non_send_resource_size = world.storages().non_send_resources.len();
    let table_size = world.storages().tables.len();
    let sparse_set_size = world.storages().sparse_sets.len();

    writeln!(result, "world:")?;
    writeln!(result, "  summary:")?;
    writeln!(result, "    component: {bundle_size}")?;
    writeln!(result, "    bundle: {component_size}")?;
    writeln!(result, "    archetype: {archetype_size}")?;
    writeln!(result, "    entity: {entity_size}")?;
    writeln!(result, "    resource: {resource_size}")?;
    writeln!(result, "    resource(non send): {non_send_resource_size}")?;
    writeln!(result, "    table: {table_size}")?;
    writeln!(result, "    sparse set: {sparse_set_size}")?;

    writeln!(result, "  detail:")?;
    let bundles = world.bundles().iter().collect::<Vec<_>>();
    if bundle_size > 0 {
        writeln!(result, "    bundles:")?;
        for bundle in bundles {
            writeln!(
                result,
                "      {:?}: {:?}",
                bundle.id(),
                bundle
                    .components()
                    .iter()
                    .map(|id| component_display(*id, world))
                    .collect::<Vec<_>>()
            )?;
        }
    }

    if component_size > 0 {
        writeln!(result, "    components:")?;

        for component in world.components().iter() {
            writeln!(
                result,
                "      {:?}({name}) {storage_type:?} {send_sync}",
                component.id(),
                name = component.name(),
                storage_type = component.storage_type(),
                send_sync = if component.is_send_and_sync() {
                    "send_sync"
                } else {
                    "non_send_sync"
                }
            )?;
        }
    }

    if archetype_size > 0 {
        writeln!(result, "    archetypes:")?;
        for archetype in world.archetypes().iter() {
            writeln!(
                result,
                "      {:?}: components:{:?} table:{:?} entity:{}",
                archetype.id(),
                archetype
                    .components()
                    .map(|id| component_display(id, world))
                    .collect::<Vec<_>>(),
                archetype.table_id(),
                archetype.len(),
            )?;
        }
    }

    if table_size > 0 {
        writeln!(result, "    tables:")?;
        for (idx, table) in world.storages().tables.iter().enumerate() {
            writeln!(result, "      [{idx}] entities: {}", table.entity_count())?;
        }
    }

    if resource_size > 0 {
        writeln!(result, "    resources:")?;
        for (component_id, _resource_data) in world.storages().resources.iter() {
            writeln!(result, "      {}", component_display(component_id, world))?;
        }
    }

    if non_send_resource_size > 0 {
        writeln!(result, "    resources(non send):")?;
        for (component_id, _resource_data) in world.storages().non_send_resources.iter() {
            let component = world.components().get_info(component_id).unwrap();
            writeln!(result, "      {:?}({})", component_id, component.name(),)?;
        }
    }

    if sparse_set_size > 0 {
        writeln!(result, "    sparse_set:")?;
        for (component_id, sparse_set) in world.storages().sparse_sets.iter() {
            let component_display = component_display(component_id, world);
            writeln!(
                result,
                "      {} entity:{}",
                component_display,
                sparse_set.len()
            )?;
        }
    }

    if let Some(schedules) = world.get_resource::<Schedules>() {
        result.push_str(&diagnostic_schedules(schedules)?);
    }

    Ok(result)
}

fn diagnostic_schedules(schedules: &Schedules) -> Result<String, std::fmt::Error> {
    let mut result = "Schedule:\n".to_string();

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

        {
            // schedule graphs
            let schedule_graph = schedule.graph();

            writeln!(
                result,
                "{}",
                diagnose_dag("hierachy", schedule_graph.hierarchy(), &id_to_names, "  ")?
                    .trim_end()
            )?;

            writeln!(
                result,
                "{}",
                diagnose_dag(
                    "dependency",
                    schedule_graph.dependency(),
                    &id_to_names,
                    "  "
                )?
                .trim_end()
            )?;

            writeln!(
                result,
                "{}",
                diagnose_dag(
                    "dependency flatten",
                    schedule_graph.dependency_flatten(),
                    &id_to_names,
                    "  "
                )?
                .trim_end()
            )?;
        }
    }

    Ok(result)
}

fn diagnose_dag(
    name: &str,
    dag: &Dag,
    id_to_names: &HashMap<NodeId, Cow<'static, str>>,
    prefix: &str,
) -> Result<String, std::fmt::Error> {
    let mut result = "".to_string();
    writeln!(result, "{prefix}{name}:")?;

    writeln!(result, "{prefix}  nodes:")?;
    for node_id in dag.graph().nodes() {
        let name = id_to_names.get(&node_id).unwrap();
        writeln!(result, "{prefix}    {node_id:?}({name})")?;
    }

    writeln!(result, "{prefix}  edges:")?;
    for (l, r, _) in dag.graph().all_edges() {
        let l_name = id_to_names.get(&l).unwrap();
        let r_name = id_to_names.get(&r).unwrap();
        writeln!(result, "{prefix}    {l:?}({l_name}) -> {r:?}({r_name})")?;
    }

    writeln!(result, "{prefix}  topsorted:")?;
    for (node_id, node_name) in dag
        .cached_topsort()
        .iter()
        .map(|node_id| (node_id, id_to_names.get(node_id).unwrap()))
    {
        writeln!(result, "{prefix}    {node_id:?}({node_name})")?;
    }
    Ok(result)
}

fn component_display(component_id: ComponentId, world: &World) -> String {
    let component = world.components().get_info(component_id).unwrap();
    format!("{:?}({})", component_id, component.name())
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
    schedule.add_system(
        increase_game_state_count
            .in_set(MySet::Set1)
            .before(sync_counter),
    );
    schedule.add_system(sync_counter);
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

    world.init_resource::<GameState>();

    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    let player = world.spawn(HitPoint(100)).id();
    // create an achetype with 2 table components and 1 sparse set
    world.spawn((HitPoint(100), Counter(1), HighlightFlag));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).insert((Counter(0),));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).insert((HighlightFlag,));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).despawn();
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);
}
