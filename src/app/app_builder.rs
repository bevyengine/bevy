use crate::{
    app::{system_stage, App},
    asset::*,
    core::Time,
    legion::prelude::{Runnable, Schedulable, Schedule, Universe, World},
    plugin::load_plugin,
    render::{
        render_graph::{
            draw_targets::*, passes::*, pipelines::*, renderers::wgpu_renderer::WgpuRenderer,
            resource_name, resource_providers::*, CompiledShaderMap, PipelineDescriptor,
            RenderGraphBuilder, Renderer, ShaderPipelineAssignments, StandardMaterial,
        },
        *,
    },
    ui,
};

use bevy_transform::{prelude::LocalToWorld, transform_system_bundle};
use std::collections::HashMap;

pub struct AppBuilder {
    pub world: World,
    pub universe: Universe,
    pub renderer: Option<Box<dyn Renderer>>,
    pub render_graph_builder: RenderGraphBuilder,
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
            render_graph_builder: RenderGraphBuilder::new(),
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

    pub fn add_default_resources(mut self) -> Self {
        let resources = &mut self.world.resources;
        resources.insert(Time::new());
        resources.insert(AssetStorage::<Mesh>::new());
        resources.insert(AssetStorage::<Texture>::new());
        resources.insert(AssetStorage::<Shader>::new());
        resources.insert(AssetStorage::<PipelineDescriptor>::new());
        resources.insert(ShaderPipelineAssignments::new());
        resources.insert(CompiledShaderMap::new());
        self
    }

    pub fn add_default_systems(mut self) -> Self {
        self = self.add_system(ui::ui_update_system::build_ui_update_system());
        for transform_system in transform_system_bundle::build(&mut self.world).drain(..) {
            self = self.add_system(transform_system);
        }

        self
    }

    pub fn add_render_graph_defaults(self) -> Self {
        self.setup_render_graph(|builder, pipeline_storage, shader_storage| {
            builder
                .add_draw_target(resource_name::draw_target::MESHES, meshes_draw_target)
                .add_draw_target(
                    resource_name::draw_target::ASSIGNED_MESHES,
                    assigned_meshes_draw_target,
                )
                .add_draw_target(resource_name::draw_target::UI, ui_draw_target)
                .add_resource_provider(Box::new(CameraResourceProvider))
                .add_resource_provider(Box::new(Camera2dResourceProvider))
                .add_resource_provider(Box::new(LightResourceProvider::new(10)))
                .add_resource_provider(Box::new(UiResourceProvider::new()))
                .add_resource_provider(Box::new(UniformResourceProvider::<StandardMaterial>::new()))
                .add_resource_provider(Box::new(UniformResourceProvider::<LocalToWorld>::new()))
                .add_forward_pass()
                .add_forward_pipeline(pipeline_storage, shader_storage)
                .add_ui_pipeline(pipeline_storage, shader_storage)
        })
    }

    pub fn setup_render_graph(
        mut self,
        setup: impl Fn(
            RenderGraphBuilder,
            &mut AssetStorage<PipelineDescriptor>,
            &mut AssetStorage<Shader>,
        ) -> RenderGraphBuilder,
    ) -> Self {
        {
            let mut pipeline_storage = self
                .world
                .resources
                .get_mut::<AssetStorage<PipelineDescriptor>>()
                .unwrap();
            let mut shader_storage = self
                .world
                .resources
                .get_mut::<AssetStorage<Shader>>()
                .unwrap();
            self.render_graph_builder = setup(
                self.render_graph_builder,
                &mut pipeline_storage,
                &mut shader_storage,
            );
        }

        self
    }

    pub fn add_wgpu_renderer(mut self) -> Self {
        self.renderer = Some(Box::new(WgpuRenderer::new()));
        self
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
