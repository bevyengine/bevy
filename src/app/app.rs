use crate::{app::AppBuilder, core::Time, render::renderer::Renderer};
use legion::prelude::*;

pub struct App {
    pub universe: Universe,
    pub world: World,
    pub resources: Resources,
    pub run: Option<Box<dyn Fn(App)>>,
    pub renderer: Option<Box<dyn Renderer>>,
    pub schedule: Schedule,
}

impl App {
    pub fn new(
        universe: Universe,
        world: World,
        schedule: Schedule,
        resources: Resources,
        run: Option<Box<dyn Fn(App)>>,
        renderer: Option<Box<dyn Renderer>>,
    ) -> App {
        App {
            universe,
            world,
            schedule,
            renderer,
            run,
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

        if let Some(ref mut renderer) = self.renderer {
            renderer.update(&mut self.world, &mut self.resources);
        }

        if let Some(mut time) = self.resources.get_mut::<Time>() {
            time.stop();
        }
    }

    pub fn run(mut self) {
        if let Some(run) = self.run.take() {
            run(self)
        }
    }
}
