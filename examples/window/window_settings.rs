use bevy::prelude::*;

/// This example illustrates how to customize the default window settings
fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "I am a window!".to_string(),
            width: 300,
            height: 300,
            vsync: true,
        })
        .add_default_plugins()
        .run();
}
