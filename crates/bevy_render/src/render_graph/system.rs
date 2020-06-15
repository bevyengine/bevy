use super::RenderGraph;
use legion::prelude::{Resources, World};

pub fn render_graph_schedule_executor_system(world: &mut World, resources: &mut Resources) {
    // run render graph systems
    let mut system_executor = {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.take_executor()
    };

    if let Some(executor) = system_executor.as_mut() {
        executor.execute(world, resources);
    }
    let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
    if let Some(executor) = system_executor.take() {
        render_graph.set_executor(executor);
    }
}
