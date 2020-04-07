use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_system_fn("hello", |_| println!("hello world!"))
        .run();
}
