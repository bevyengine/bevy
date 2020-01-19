use crate::{
    app::App,
    asset::*,
    core::Time,
    legion::prelude::{Runnable, Schedulable, Schedule, Universe, World},
    render::{passes::*, *},
    ui,
};

use bevy_transform::transform_system_bundle;
use std::collections::HashMap;

pub const UPDATE: &str = "update";

pub struct AppBuilder {
    pub world: World,
    pub universe: Universe,
    pub render_graph: RenderGraph,
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
            render_graph: RenderGraph::new(),
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
            self.render_graph,
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
        self.add_system_to_stage(UPDATE, system)
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

    pub fn add_default_passes(mut self) -> Self {
        let msaa_samples = 4;
        let render_graph = &mut self.render_graph;
        render_graph
            .add_render_resource_manager(Box::new(render_resources::MaterialResourceManager));
        render_graph
            .add_render_resource_manager(Box::new(render_resources::LightResourceManager::new(10)));
        render_graph.add_render_resource_manager(Box::new(render_resources::GlobalResourceManager));
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

    pub fn add_defaults(self) -> Self {
        self.add_default_resources()
            .add_default_passes()
            .add_default_systems()
    }
}
