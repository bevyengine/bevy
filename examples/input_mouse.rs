use bevy::{input::mouse::MouseInput, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .build_system(mouse_input_system)
        .run();
}

pub fn mouse_input_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut mouse_input_event_reader = resources.get_event_reader::<MouseInput>();
    SystemBuilder::new("mouse_input")
        .read_resource::<Events<MouseInput>>()
        .build(
            move |_command_buffer, _world, mouse_input_events, _queries| {
                for event in mouse_input_events.iter(&mut mouse_input_event_reader) {
                    println!("{:?}", event);
                }
            },
        )
}