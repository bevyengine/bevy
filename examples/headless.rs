use bevy::{
    app::schedule_runner::{RunMode, ScheduleRunnerPlugin},
    prelude::*,
};
use std::time::Duration;

fn main() {
    println!("This app runs once:");
    App::build()
        .add_plugin(ScheduleRunnerPlugin {
            run_mode: RunMode::Once,
        })
        .add_system(hello_world_system())
        .run();

    println!("This app loops forever at 60 fps:");
    App::build()
        .add_plugin(ScheduleRunnerPlugin {
            run_mode: RunMode::Loop {
                wait: Some(Duration::from_secs_f64(1.0 / 60.0)),
            },
        })
        .add_system(hello_world_system())
        .run();
}

pub fn hello_world_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("hello_world").build(move |_, _, _, _| {
        println!("hello world");
    })
}
