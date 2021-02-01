use bevy::{prelude::*, utils::Duration};

/// Plugins are the foundation of Bevy. They are scoped sets of components, resources, and systems
/// that provide a specific piece of functionality (generally the smaller the scope, the better).
/// This example illustrates how to create a simple plugin that prints out a message.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
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

impl Plugin for PrintMessagePlugin {
    // this is where we set up our plugin
    fn build(&self, app: &mut AppBuilder) {
        let state = PrintMessageState {
            message: self.message.clone(),
            timer: Timer::new(self.wait_duration, true),
        };
        app.insert_resource(state)
            .add_system(print_message_system.system());
    }
}

struct PrintMessageState {
    message: String,
    timer: Timer,
}

fn print_message_system(mut state: ResMut<PrintMessageState>, time: Res<Time>) {
    if state.timer.tick(time.delta_seconds()).finished() {
        println!("{}", state.message);
    }
}
