use crate::{
    app::{system_stage, App},
    core::{plugin::{AppPlugin, load_plugin}, winit::WinitPlugin, CorePlugin},
    legion::prelude::{Resources, Runnable, Schedulable, Schedule, Universe, World},
    render::{renderer::Renderer, *},
    ui,
};

use bevy_transform::transform_system_bundle;
use render_resource::build_entity_render_resource_assignments_system;
use std::collections::HashMap;

pub struct AppBuilder {
    pub world: World,
    pub resources: Resources,
    pub universe: Universe,
    pub renderer: Option<Box<dyn Renderer>>,
    pub run: Option<Box<dyn Fn(App)>>,
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
            renderer: None,
            run: None,
            setup_systems: Vec::new(),
            system_stages: HashMap::new(),
            runnable_stages: HashMap::new(),
            stage_order: Vec::new(),
        }
    }

    pub fn build(mut self) -> App {
        let mut setup_schedule_builder = Schedule::builder();
        for setup_system in self.setup_systems.drain(..) {
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

        App::new(
            self.universe,
            self.world,
            self.resources,
            schedule_builder.build(),
            self.run.take(),
            self.renderer.take(),
        )
    }

    pub fn run(self) {
        self.build().run();
    }

    pub fn with_world(mut self, world: World) -> Self {
        self.world = world;
        self
    }

    pub fn setup(mut self, setup: impl Fn(&mut World, &mut Resources)) -> Self {
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

    pub fn add_resource<T>(mut self, resource: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.resources.insert(resource);
        self
    }

    pub fn add_default_systems(mut self) -> Self {
        self = self
            .add_system(build_entity_render_resource_assignments_system())
            .add_system(ui::ui_update_system::build_ui_update_system());
        for transform_system in transform_system_bundle::build(&mut self.world).drain(..) {
            self = self.add_system(transform_system);
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

    pub fn add_defaults(mut self) -> Self {
        self = self
            .add_default_systems()
            .add_plugin(CorePlugin::default())
            .add_plugin(RenderPlugin::default());

        #[cfg(feature = "wgpu")]
        {
            self = self.add_wgpu_renderer();
        }
        #[cfg(feature = "winit")]
        {
            self = self.add_plugin(WinitPlugin::default())
        }
        self
    }

    pub fn load_plugin(self, path: &str) -> Self {
        let (_lib, plugin) = load_plugin(path);
        plugin.build(self)
    }

    pub fn add_plugin<T>(self, plugin: T) -> Self
    where
        T: AppPlugin,
    {
        plugin.build(self)
    }
}
