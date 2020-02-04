use crate::{
    app::{App, system_stage},
    asset::*,
    core::Time,
    legion::prelude::{Runnable, Schedulable, Schedule, Universe, World},
    render::render_graph_2,
    render::render_graph_2::{pipelines::*, wgpu_renderer::WgpuRenderer, resource_provider::CameraResourceProvider, UniformResourceProvider, StandardMaterial},
    render::{passes::*, *},
    plugin::load_plugin,
    ui,
};

use bevy_transform::{transform_system_bundle, prelude::LocalToWorld};
use std::collections::HashMap;

pub struct AppBuilder {
    pub world: World,
    pub universe: Universe,
    pub legacy_render_graph: Option<RenderGraph>,
    pub renderer: Option<Box<dyn render_graph_2::Renderer>>,
    pub render_graph_builder: render_graph_2::RenderGraphBuilder,
    pub system_stages: HashMap<String, Vec<Box<dyn Schedulable>>>,
    pub runnable_stages: HashMap<String, Vec<Box<dyn Runnable>>>,
    pub stage_order: Vec<String>,
}

impl AppBuilder {
    pub fn new() -> Self {
        let universe = Universe::new();
        let world = universe.create_world();
        AppBuilder {
            universe,
            world,
            render_graph_builder: render_graph_2::RenderGraphBuilder::new(),
            legacy_render_graph: None,
            renderer: None,
            system_stages: HashMap::new(),
            runnable_stages: HashMap::new(),
            stage_order: Vec::new(),
        }
    }

    pub fn build(mut self) -> App {
        let mut schedule_builder = Schedule::builder();
        for stage_name in self.stage_order.iter() {
            if let Some((_name, stage_systems)) = self.system_stages.remove_entry(stage_name) {
                for system in stage_systems {
                    schedule_builder = schedule_builder.add_system(system);
                }

                schedule_builder = schedule_builder.flush();
            }

            if let Some((_name, stage_runnables)) = self.runnable_stages.remove_entry(stage_name) {
                for system in stage_runnables {
                    schedule_builder = schedule_builder.add_thread_local(system);
                }

                schedule_builder = schedule_builder.flush();
            }
        }

        App::new(
            self.universe,
            self.world,
            schedule_builder.build(),
            self.legacy_render_graph,
            self.renderer,
            self.render_graph_builder.build(),
        )
    }

    pub fn run(self) {
        self.build().run();
    }

    pub fn with_world(mut self, world: World) -> Self {
        self.world = world;
        self
    }

    pub fn setup_world(mut self, setup: impl Fn(&mut World)) -> Self {
        setup(&mut self.world);
        self
    }

    pub fn add_system(self, system: Box<dyn Schedulable>) -> Self {
        self.add_system_to_stage(system_stage::UPDATE, system)
    }

    pub fn add_system_to_stage(mut self, stage_name: &str, system: Box<dyn Schedulable>) -> Self {
        if let None = self.system_stages.get(stage_name) {
            self.system_stages
                .insert(stage_name.to_string(), Vec::new());
            self.stage_order.push(stage_name.to_string());
        }

        let stages = self.system_stages.get_mut(stage_name).unwrap();
        stages.push(system);

        self
    }

    pub fn add_runnable_to_stage(mut self, stage_name: &str, system: Box<dyn Runnable>) -> Self {
        if let None = self.runnable_stages.get(stage_name) {
            self.runnable_stages
                .insert(stage_name.to_string(), Vec::new());
            self.stage_order.push(stage_name.to_string());
        }

        let stages = self.runnable_stages.get_mut(stage_name).unwrap();
        stages.push(system);

        self
    }

    pub fn with_legacy_render_graph(mut self) -> Self {
        self.legacy_render_graph = Some(RenderGraph::new());
        self
    }

    pub fn add_default_passes(mut self) -> Self {
        let msaa_samples = 4;
        {
            let render_graph = &mut self.legacy_render_graph.as_mut().unwrap();
            render_graph
                .add_render_resource_manager(Box::new(render_resources::MaterialResourceManager));
            render_graph.add_render_resource_manager(Box::new(
                render_resources::LightResourceManager::new(10),
            ));
            render_graph
                .add_render_resource_manager(Box::new(render_resources::GlobalResourceManager));
            render_graph
                .add_render_resource_manager(Box::new(render_resources::Global2dResourceManager));

            let depth_format = wgpu::TextureFormat::Depth32Float;
            render_graph.set_pass(
                "forward",
                Box::new(ForwardPass::new(depth_format, msaa_samples)),
            );
            render_graph.set_pipeline(
                "forward",
                "forward",
                Box::new(ForwardPipeline::new(msaa_samples)),
            );
            render_graph.set_pipeline(
                "forward",
                "forward_instanced",
                Box::new(ForwardInstancedPipeline::new(depth_format, msaa_samples)),
            );
            render_graph.set_pipeline("forward", "ui", Box::new(UiPipeline::new(msaa_samples)));
        }

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
        self = self.add_system(ui::ui_update_system::build_ui_update_system());
        for transform_system in transform_system_bundle::build(&mut self.world).drain(..) {
            self = self.add_system(transform_system);
        }

        self
    }

    pub fn add_render_graph_defaults(mut self) -> Self {
        self.render_graph_builder = self
            .render_graph_builder
            .add_resource_provider(Box::new(CameraResourceProvider))
            .add_resource_provider(Box::new(UniformResourceProvider::<StandardMaterial>::new()))
            .add_resource_provider(Box::new(UniformResourceProvider::<LocalToWorld>::new()))
            .add_forward_pass()
            .add_forward_pipeline();

        self
    }

    pub fn add_wgpu_renderer(mut self) -> Self {
        self.renderer = Some(Box::new(WgpuRenderer::new()));
        self
    }

    pub fn add_defaults_legacy(self) -> Self {
        self.with_legacy_render_graph()
            .add_default_resources()
            .add_default_passes()
            .add_default_systems()
    }

    pub fn add_defaults(self) -> Self {
        self.add_default_resources()
            .add_default_systems()
            .add_render_graph_defaults()
            .add_wgpu_renderer()
    }

    pub fn load_plugin(mut self, path: &str) -> Self {
        let (_lib, plugin) = load_plugin(path);
        self = plugin.build(self);
        self
    }
}
