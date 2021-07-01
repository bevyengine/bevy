use crate::app_builder::AppBuilder;
use bevy_ecs::{
    schedule::{Schedule, Stage},
    world::World,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

#[allow(clippy::needless_doctest_main)]
/// Containers of app logic and data
///
/// App store the ECS World, Resources, Schedule, and Executor. They also store the "run" function
/// of the App, which by default executes the App schedule once. Apps are constructed using the
/// builder pattern.
///
/// ## Example
/// Here is a simple "Hello World" Bevy app:
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_ecs::prelude::*;
///
/// fn main() {
///    App::build()
///        .add_system(hello_world_system.system())
///        .run();
/// }
///
/// fn hello_world_system() {
///    println!("hello world");
/// }
/// ```
pub struct App {
    pub world: World,
    pub runner: Box<dyn Fn(App)>,
    pub schedule: Schedule,
    #[cfg(feature = "trace")]
    frame_count: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            world: Default::default(),
            schedule: Default::default(),
            runner: Box::new(run_once),
            #[cfg(feature = "trace")]
            frame_count: 0,
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
        #[cfg(feature = "trace")]
        {
            self.frame_count = self.frame_count.wrapping_add(1);
            let bevy_frame_update_span = info_span!("frame", frame_count = self.frame_count);
            let _bevy_frame_update_guard = bevy_frame_update_span.enter();
            self.schedule
                .run_in_span(&mut self.world, Some(&bevy_frame_update_span));
        }
        #[cfg(not(feature = "trace"))]
        self.schedule.run(&mut self.world);
    }

    pub fn run(mut self) {
        #[cfg(feature = "trace")]
        let bevy_app_run_span = info_span!("bevy_app");
        #[cfg(feature = "trace")]
        let _bevy_app_run_guard = bevy_app_run_span.enter();

        let runner = std::mem::replace(&mut self.runner, Box::new(run_once));
        (runner)(self);
    }
}

/// An event that indicates the app should exit. This will fully exit the app process.
#[derive(Debug, Clone)]
pub struct AppExit;
