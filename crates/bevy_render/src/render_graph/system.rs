use super::RenderGraph;
use bevy_ecs::{schedule::Stage, world::World};

pub fn render_graph_schedule_executor_system(world: &mut World) {
    // run render graph systems
    let mut system_schedule = {
        let mut render_graph = world.get_resource_mut::<RenderGraph>().unwrap();
        render_graph.take_schedule()
    };

    if let Some(schedule) = system_schedule.as_mut() {
        schedule.run(world);
    }
    let mut render_graph = world.get_resource_mut::<RenderGraph>().unwrap();
    if let Some(schedule) = system_schedule.take() {
        render_graph.set_schedule(schedule);
    }
}
