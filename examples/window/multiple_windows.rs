use bevy::{prelude::*, window::CreateWindow};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(mut create_window_events: ResourceMut<Events<CreateWindow>>) {
    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        descriptor: WindowDescriptor {
            width: 800,
            height: 600,
            vsync: false,
            title: "another window".to_string(),
        },
    });
}
