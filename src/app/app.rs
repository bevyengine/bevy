use crate::{app::AppBuilder, core::Time};
use legion::prelude::*;

pub struct App {
    pub universe: Universe,
    pub world: World,
    pub resources: Resources,
    pub runner: Option<Box<dyn Fn(App)>>,
    pub schedule: Schedule,
}

impl App {
    pub fn new(
        universe: Universe,
        world: World,
        resources: Resources,
        schedule: Schedule,
        run: Option<Box<dyn Fn(App)>>,
    ) -> App {
        App {
            universe,
            world,
            schedule,
            runner: run,
            resources,
        }
    }

    pub fn build() -> AppBuilder {
        AppBuilder::new()
    }

    pub fn update(&mut self) {
        if let Some(mut time) = self.resources.get_mut::<Time>() {
            time.start();
        }
        self.schedule.execute(&mut self.world, &mut self.resources);

        if let Some(mut time) = self.resources.get_mut::<Time>() {
            time.stop();
        }
    }

    pub fn run(mut self) {
        if let Some(run) = self.runner.take() {
            run(self)
        }
    }
}
