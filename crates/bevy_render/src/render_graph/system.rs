use super::RenderGraph;
use bevy_ecs::{Resources, World};

pub fn render_graph_schedule_executor_system(world: &mut World, resources: &mut Resources) {
    // run render graph systems
    let (mut system_schedule, mut commands) = {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        (render_graph.take_schedule(), render_graph.take_commands())
    };

    commands.apply(world, resources);
    if let Some(schedule) = system_schedule.as_mut() {
        schedule.initialize_and_run(world, resources);
    }
    let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
    if let Some(schedule) = system_schedule.take() {
        render_graph.set_schedule(schedule);
    }
}
