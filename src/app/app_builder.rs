use crate::{App, asset::AssetStorage, legion::prelude::{World, SystemScheduler}, render::{*, passes::*}, AppStage, Time};

pub struct AppBuilder {
    pub app: App,
}

impl AppBuilder {
    pub fn new() -> Self {
        let world = World::new();
        let scheduler = SystemScheduler::<AppStage>::new();
        AppBuilder {
            app: App::new(world, scheduler),
        }
    }

    pub fn build(self) -> App {
        self.app
    }

    pub fn run(self) {
        self.app.run();
    }

    pub fn setup(mut self, setup: &dyn Fn(&mut World, &mut SystemScheduler<AppStage>)) -> Self {
        setup(&mut self.app.world, &mut self.app.scheduler);
        self
    }

    pub fn add_default_passes(mut self) -> Self {
        self.app.render_graph.add_render_resource_manager(Box::new(render_resources::MaterialResourceManager));
        self.app.render_graph.add_render_resource_manager(Box::new(render_resources::LightResourceManager::new(10)));
        self.app.render_graph.add_render_resource_manager(Box::new(render_resources::GlobalResourceManager));
        self.app.render_graph.add_render_resource_manager(Box::new(render_resources::Global2dResourceManager));

        let depth_format = wgpu::TextureFormat::Depth32Float;
        self.app.render_graph.set_pass("forward", Box::new(ForwardPass::new(depth_format)));
        self.app.render_graph.set_pipeline("forward", "forward", Box::new(ForwardPipeline::new()));
        self.app.render_graph.set_pipeline("forward", "forward_instanced", Box::new(ForwardInstancedPipeline::new(depth_format)));
        self.app.render_graph.set_pipeline("forward", "ui", Box::new(UiPipeline::new()));

        self
    }

    pub fn add_default_resources(mut self) -> Self {
        self.app.world.resources.insert(Time::new());
        self.app.world.resources.insert(AssetStorage::<Mesh, MeshType>::new());
        self
    }

    pub fn add_defaults(self) -> Self {
        self
            .add_default_resources()
            .add_default_passes()
    }
}