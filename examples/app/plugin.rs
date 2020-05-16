use bevy::prelude::*;
use std::time::Duration;

// Plugins are the foundation of Bevy. They are scoped sets of components, resources, and systems
// that provide a specific piece of functionality (generally the smaller the scope, the better).
fn main() {
    App::build()
        .add_default_plugins()
        // plugins are registered as part of the "app building" process
        .add_plugin(PrintMessagePlugin {
            wait_duration: Duration::from_secs(1),
            message: "This is an example plugin".to_string(),
        })
        .run();
}

// This "print message plugin" prints a `message` every `wait_duration`
pub struct PrintMessagePlugin {
    // Put your plugin configuration here
    wait_duration: Duration,
    message: String,
}

impl AppPlugin for PrintMessagePlugin {
    // this is where we set up our plugin
    fn build(&self, app: &mut AppBuilder) {
        let state = PrintMessageState {
            message: self.message.clone(),
            elapsed_time: 0.0,
            duration: self.wait_duration,
        };
        app.add_resource(state)
            .add_system(print_message_system.system());
    }
}

struct PrintMessageState {
    message: String,
    duration: Duration,
    elapsed_time: f32,
}

fn print_message_system(time: Res<Time>, mut state: ResMut<PrintMessageState>) {
    state.elapsed_time += time.delta_seconds;
    if state.elapsed_time > state.duration.as_secs_f32() {
        println!("{}", state.message);
        state.elapsed_time = 0.0;
    }
}
