use super::AppBuilder;
use bevy_ecs::{Resources, Schedule, World};

#[derive(Default)]
pub struct App {
    pub world: World,
    pub resources: Resources,
    pub runner: Option<Box<dyn Fn(App)>>,
    pub schedule: Schedule,
    pub startup_schedule: Schedule,
}

impl App {
    pub fn build() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn update(&mut self) {
        self.schedule.initialize(&mut self.resources);
        self.schedule.run(&mut self.world, &mut self.resources);
    }

    pub fn run(mut self) {
        self.startup_schedule.initialize(&mut self.resources);
        self.startup_schedule.run(&mut self.world, &mut self.resources);
        if let Some(run) = self.runner.take() {
            run(self)
        }
    }
}

/// An event that indicates the app should exit. This will fully exit the app process.
pub struct AppExit;
