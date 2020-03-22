use crate::{
    app::{system_stage, App},
    asset::*,
    core::Time,
    legion::prelude::{Resources, Runnable, Schedulable, Schedule, Universe, World},
    plugin::load_plugin,
    prelude::StandardMaterial,
    render::{
        draw_target::draw_targets::*, mesh::Mesh, pass::passes::*, pipeline::pipelines::*,
        render_resource::resource_providers::*, renderer::Renderer, texture::Texture, *,
    },
    ui,
};

use bevy_transform::{prelude::LocalToWorld, transform_system_bundle};
use pipeline::PipelineDescriptor;
use render_graph::RenderGraphBuilder;
use render_resource::{
    AssetBatchers, EntityRenderResourceAssignments, RenderResourceAssignmentsProvider,
};
use shader::Shader;
use std::collections::HashMap;

pub struct AppBuilder {
    pub world: World,
    pub resources: Resources,
    pub universe: Universe,
    pub renderer: Option<Box<dyn Renderer>>,
    pub render_graph_builder: RenderGraphBuilder,
    pub setup_systems: Vec<Box<dyn Schedulable>>,
    pub system_stages: HashMap<String, Vec<Box<dyn Schedulable>>>,
    pub runnable_stages: HashMap<String, Vec<Box<dyn Runnable>>>,
    pub stage_order: Vec<String>,
}

impl AppBuilder {
    pub fn new() -> Self {
        let universe = Universe::new();
        let world = universe.create_world();
        let resources = Resources::default();
        AppBuilder {
            universe,
            world,
            resources,
            render_graph_builder: RenderGraphBuilder::new(),
            renderer: None,
            setup_systems: Vec::new(),
            system_stages: HashMap::new(),
            runnable_stages: HashMap::new(),
            stage_order: Vec::new(),
        }
    }

    pub fn build(mut self) -> App {
        let mut setup_schedule_builder = Schedule::builder();
        for setup_system in self.setup_systems {
            setup_schedule_builder = setup_schedule_builder.add_system(setup_system);    
        }

        let mut setup_schedule = setup_schedule_builder.build();
        setup_schedule.execute(&mut self.world, &mut self.resources);

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

        let render_graph = self.render_graph_builder.build();
        self.resources.insert(render_graph);

        App::new(
            self.universe,
            self.world,
            schedule_builder.build(),
            self.resources,
            self.renderer,
        )
    }

    pub fn run(self) {
        self.build().run();
    }

    pub fn with_world(mut self, world: World) -> Self {
        self.world = world;
        self
    }

    pub fn setup_world(mut self, setup: impl Fn(&mut World, &mut Resources)) -> Self {
        setup(&mut self.world, &mut self.resources);
        self
    }

    pub fn add_system(self, system: Box<dyn Schedulable>) -> Self {
        self.add_system_to_stage(system_stage::UPDATE, system)
    }

    pub fn add_setup_system(mut self, system: Box<dyn Schedulable>) -> Self {
        self.setup_systems.push(system);
        self
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
        let mut asset_batchers = AssetBatchers::default();
        asset_batchers.batch_types2::<Mesh, StandardMaterial>();
        self.resources.insert(Time::new());
        self.resources.insert(AssetStorage::<Mesh>::new());
        self.resources.insert(AssetStorage::<Texture>::new());
        self.resources.insert(AssetStorage::<Shader>::new());
        self.resources
            .insert(AssetStorage::<StandardMaterial>::new());
        self.resources
            .insert(AssetStorage::<PipelineDescriptor>::new());
        self.resources.insert(ShaderPipelineAssignments::new());
        self.resources.insert(CompiledShaderMap::new());
        self.resources
            .insert(RenderResourceAssignmentsProvider::default());
        self.resources
            .insert(EntityRenderResourceAssignments::default());
        self.resources.insert(asset_batchers);
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
                .add_draw_target(MeshesDrawTarget::default())
                .add_draw_target(AssignedBatchesDrawTarget::default())
                .add_draw_target(AssignedMeshesDrawTarget::default())
                .add_draw_target(UiDrawTarget::default())
                .add_resource_provider(CameraResourceProvider::default())
                .add_resource_provider(Camera2dResourceProvider::default())
                .add_resource_provider(LightResourceProvider::new(10))
                .add_resource_provider(UiResourceProvider::new())
                .add_resource_provider(MeshResourceProvider::new())
                .add_resource_provider(UniformResourceProvider::<StandardMaterial>::new())
                .add_resource_provider(UniformResourceProvider::<LocalToWorld>::new())
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
                .resources
                .get_mut::<AssetStorage<PipelineDescriptor>>()
                .unwrap();
            let mut shader_storage = self.resources.get_mut::<AssetStorage<Shader>>().unwrap();
            self.render_graph_builder = setup(
                self.render_graph_builder,
                &mut pipeline_storage,
                &mut shader_storage,
            );
        }

        self
    }

    #[cfg(feature = "wgpu")]
    pub fn add_wgpu_renderer(mut self) -> Self {
        self.renderer = Some(Box::new(
            renderer::renderers::wgpu_renderer::WgpuRenderer::new(),
        ));
        self
    }

    #[cfg(not(feature = "wgpu"))]
    fn add_wgpu_renderer(self) -> Self {
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
