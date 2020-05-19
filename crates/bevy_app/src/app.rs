use super::AppBuilder;
use legion::prelude::*;

pub struct App {
    pub world: World,
    pub resources: Resources,
    pub runner: Option<Box<dyn Fn(App)>>,
    pub schedule: Option<Schedule>,
}

impl Default for App {
    fn default() -> Self {
        let universe = Universe::new();
        let world = universe.create_world();
        let mut resources = Resources::default();
        resources.insert(universe);
        App {
            world,
            resources,
            runner: None,
            schedule: None,
        }
    }
}

impl App {
    pub fn build() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn update(&mut self) {
        if let Some(ref mut schedule) = self.schedule {
            schedule.execute(&mut self.world, &mut self.resources);
        }
    }

    pub fn run(mut self) {
        if let Some(run) = self.runner.take() {
            run(self)
        }
    }
}
