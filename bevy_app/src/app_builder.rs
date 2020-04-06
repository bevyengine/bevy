use crate::{
    plugin::{load_plugin, AppPlugin},
    system_stage, App, Events,
};

use legion::prelude::{Resources, Runnable, Schedulable, Schedule, Universe, World};
use std::collections::HashMap;

static APP_MISSING_MESSAGE: &str = "This AppBuilder no longer has an App. Check to see if you already called run(). A call to app_builder.run() consumes the AppBuilder's App.";

pub struct AppBuilder {
    app: Option<App>,
    pub setup_systems: Vec<Box<dyn Schedulable>>,
    // TODO: these separate lists will produce incorrect ordering
    pub system_stages: HashMap<String, Vec<Box<dyn Schedulable>>>,
    pub runnable_stages: HashMap<String, Vec<Box<dyn Runnable>>>,
    pub thread_local_stages: HashMap<String, Vec<Box<dyn FnMut(&mut World, &mut Resources)>>>,
    pub stage_order: Vec<String>,
}

impl AppBuilder {
    pub fn new() -> Self {
        AppBuilder {
            app: Some(App::default()),
            setup_systems: Vec::new(),
            system_stages: HashMap::new(),
            runnable_stages: HashMap::new(),
            thread_local_stages: HashMap::new(),
            stage_order: Vec::new(),
        }
    }

    pub fn app(&self) -> &App {
        self.app.as_ref().expect(APP_MISSING_MESSAGE)
    }

    pub fn app_mut(&mut self) -> &mut App {
        self.app.as_mut().expect(APP_MISSING_MESSAGE)
    }

    pub fn world(&self) -> &World {
        &self.app().world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.app_mut().world
    }

    pub fn universe(&self) -> &Universe {
        &self.app().universe
    }

    pub fn universe_mut(&mut self) -> &mut Universe {
        &mut self.app_mut().universe
    }

    pub fn resources(&self) -> &Resources {
        &self.app().resources
    }

    pub fn resources_mut(&mut self) -> &mut Resources {
        &mut self.app_mut().resources
    }

    pub fn build_schedule(&mut self) -> &mut Self {
        let mut setup_schedule_builder = Schedule::builder();
        for setup_system in self.setup_systems.drain(..) {
            setup_schedule_builder = setup_schedule_builder.add_system(setup_system);
        }

        let mut setup_schedule = setup_schedule_builder.build();
        let app = self.app_mut();
        setup_schedule.execute(&mut app.world, &mut app.resources);

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

            if let Some((_name, stage_thread_locals)) =
                self.thread_local_stages.remove_entry(stage_name)
            {
                for system in stage_thread_locals {
                    schedule_builder = schedule_builder.add_thread_local_fn(system);
                }

                schedule_builder = schedule_builder.flush();
            }
        }

        let app = self.app_mut();
        app.schedule = Some(schedule_builder.build());

        self
    }

    pub fn run(&mut self) {
        self.build_schedule();
        self.app.take().unwrap().run();
    }

    pub fn set_world(&mut self, world: World) -> &mut Self {
        self.app_mut().world = world;
        self
    }

    pub fn setup(&mut self, setup: impl Fn(&mut World, &mut Resources)) -> &mut Self {
        let app = self.app_mut();
        setup(&mut app.world, &mut app.resources);
        self
    }

    pub fn add_system(&mut self, system: Box<dyn Schedulable>) -> &mut Self {
        self.add_system_to_stage(system_stage::UPDATE, system)
    }

    pub fn add_setup_system(&mut self, system: Box<dyn Schedulable>) -> &mut Self {
        self.setup_systems.push(system);
        self
    }

    pub fn build_system<F>(&mut self, build: F) -> &mut Self
    where
        F: Fn(&mut Resources) -> Box<dyn Schedulable>,
    {
        let system = build(self.resources_mut());
        self.add_system(system)
    }

    pub fn add_system_to_stage(
        &mut self,
        stage_name: &str,
        system: Box<dyn Schedulable>,
    ) -> &mut Self {
        if let None = self.system_stages.get(stage_name) {
            self.system_stages
                .insert(stage_name.to_string(), Vec::new());
            self.stage_order.push(stage_name.to_string());
        }

        let stages = self.system_stages.get_mut(stage_name).unwrap();
        stages.push(system);

        self
    }

    pub fn add_runnable_to_stage(
        &mut self,
        stage_name: &str,
        system: Box<dyn Runnable>,
    ) -> &mut Self {
        if let None = self.runnable_stages.get(stage_name) {
            self.runnable_stages
                .insert(stage_name.to_string(), Vec::new());
            self.stage_order.push(stage_name.to_string());
        }

        let stages = self.runnable_stages.get_mut(stage_name).unwrap();
        stages.push(system);

        self
    }

    pub fn add_thread_local_to_stage(
        &mut self,
        stage_name: &str,
        f: impl FnMut(&mut World, &mut Resources) + 'static,
    ) -> &mut Self {
        if let None = self.thread_local_stages.get(stage_name) {
            self.thread_local_stages
                .insert(stage_name.to_string(), Vec::new());
            // TODO: this is so broken
            self.stage_order.push(stage_name.to_string());
        }

        let thread_local_stages = self.thread_local_stages.get_mut(stage_name).unwrap();
        thread_local_stages.push(Box::new(f));
        self
    }

    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.add_resource(Events::<T>::default())
            .add_system_to_stage(
                system_stage::EVENT_UPDATE,
                Events::<T>::build_update_system(),
            )
    }

    pub fn add_resource<T>(&mut self, resource: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.resources_mut().insert(resource);
        self
    }

    pub fn set_runner(&mut self, run_fn: impl Fn(App) + 'static) -> &mut Self {
        self.app_mut().runner = Some(Box::new(run_fn));
        self
    }

    pub fn load_plugin(&mut self, path: &str) -> &mut Self {
        let (_lib, plugin) = load_plugin(path);
        log::debug!("loaded plugin: {}", plugin.name());
        plugin.build(self);
        self
    }

    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: AppPlugin,
    {
        log::debug!("added plugin: {}", plugin.name());
        plugin.build(self);
        self
    }
}
