use super::AppBuilder;
use bevy_ecs::{Resources, Schedule, World, ParallelExecutor};

#[derive(Default)]
pub struct App {
    pub world: World,
    pub resources: Resources,
    pub runner: Option<Box<dyn Fn(App)>>,
    pub schedule: Schedule,
    pub executor: ParallelExecutor,
    pub startup_schedule: Schedule,
    pub startup_executor: ParallelExecutor,
}

impl App {
    pub fn build() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn update(&mut self) {
        self.schedule.initialize(&mut self.resources);
        self.executor.run(&mut self.schedule, &mut self.world, &mut self.resources);
    }

    pub fn run(mut self) {
        self.startup_schedule.initialize(&mut self.resources);
        self.startup_executor.run(&mut self.startup_schedule, &mut self.world, &mut self.resources);
        if let Some(run) = self.runner.take() {
            run(self)
        }
    }
}

/// An event that indicates the app should exit. This will fully exit the app process.
pub struct AppExit;
