use crate::app_builder::AppBuilder;
use bevy_ecs::{ParallelExecutor, Resources, Schedule, World};

pub struct App {
    pub world: World,
    pub resources: Resources,
    pub runner: Box<dyn Fn(App)>,
    pub schedule: Schedule,
    pub executor: ParallelExecutor,
    pub startup_schedule: Schedule,
    pub startup_executor: ParallelExecutor,
}

impl Default for App {
    fn default() -> Self {
        Self {
            world: Default::default(),
            resources: Default::default(),
            schedule: Default::default(),
            executor: Default::default(),
            startup_schedule: Default::default(),
            startup_executor: ParallelExecutor::without_tracker_clears(),
            runner: Box::new(run_once),
        }
    }
}

fn run_once(mut app: App) {
    app.update();
}

impl App {
    pub fn build() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn update(&mut self) {
        self.schedule.initialize(&mut self.resources);
        self.executor
            .run(&mut self.schedule, &mut self.world, &mut self.resources);
    }

    pub fn run(mut self) {
        self.startup_schedule.initialize(&mut self.resources);
        self.startup_executor.run(
            &mut self.startup_schedule,
            &mut self.world,
            &mut self.resources,
        );

        let runner = std::mem::replace(&mut self.runner, Box::new(run_once));
        (runner)(self);
    }
}

/// An event that indicates the app should exit. This will fully exit the app process.
pub struct AppExit;
