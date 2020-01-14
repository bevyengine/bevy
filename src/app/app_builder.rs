use crate::{
    asset::*,
    legion::{
        prelude::{Schedule, World},
        schedule::Builder,
    },
    render::{passes::*, *},
    transform_system_bundle, ui, App, Time,
};

pub struct AppBuilder {
    pub world: World,
    pub schedule_builder: Builder,
    pub render_graph: RenderGraph,
}

impl AppBuilder {
    pub fn new() -> Self {
        AppBuilder {
            world: World::new(),
            schedule_builder: Schedule::builder(),
            render_graph: RenderGraph::new(),
        }
    }

    pub fn build(self) -> App {
        App::new(self.world, self.schedule_builder.build(), self.render_graph)
    }

    pub fn run(self) {
        self.build().run();
    }

    pub fn with_world(mut self, world: World) -> Self {
        self.world = world;
        self
    }

    pub fn with_schedule(mut self, schedule_builder: Builder) -> Self {
        self.schedule_builder = schedule_builder;
        self
    }

    pub fn setup_world(mut self, setup: impl Fn(&mut World)) -> Self {
        setup(&mut self.world);
        self
    }

    pub fn setup_systems(mut self, setup: impl Fn(Builder) -> Builder) -> Self {
        self.schedule_builder = setup(self.schedule_builder);
        self
    }

    pub fn add_default_passes(mut self) -> Self {
        let msaa_samples = 8;
        let render_graph = &mut self.render_graph;
        render_graph
            .add_render_resource_manager(Box::new(render_resources::MaterialResourceManager));
        render_graph
            .add_render_resource_manager(Box::new(render_resources::LightResourceManager::new(10)));
        render_graph.add_render_resource_manager(Box::new(render_resources::GlobalResourceManager));
        render_graph
            .add_render_resource_manager(Box::new(render_resources::Global2dResourceManager));

        let depth_format = wgpu::TextureFormat::Depth32Float;
        render_graph.set_pass("forward", Box::new(ForwardPass::new(depth_format, msaa_samples)));
        render_graph.set_pipeline("forward", "forward", Box::new(ForwardPipeline::new(msaa_samples)));
        render_graph.set_pipeline(
            "forward",
            "forward_instanced",
            Box::new(ForwardInstancedPipeline::new(depth_format, msaa_samples)),
        );
        render_graph.set_pipeline("forward", "ui", Box::new(UiPipeline::new(msaa_samples)));

        self
    }

    pub fn add_default_resources(mut self) -> Self {
        let resources = &mut self.world.resources;
        resources.insert(Time::new());
        resources.insert(AssetStorage::<Mesh>::new());
        resources.insert(AssetStorage::<Texture>::new());
        self
    }

    pub fn add_default_systems(mut self) -> Self {
        self.schedule_builder = self
            .schedule_builder
            .add_system(ui::ui_update_system::build_ui_update_system());
        for transform_system in transform_system_bundle::build(&mut self.world).drain(..) {
            self.schedule_builder = self.schedule_builder.add_system(transform_system);
        }

        self
    }

    pub fn add_defaults(self) -> Self {
        self.add_default_resources()
            .add_default_passes()
            .add_default_systems()
    }
}
