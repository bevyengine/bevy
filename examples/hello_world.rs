use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(|_: &mut World, _: &mut Resources| {
            println!("hello world");
        })
        .run();
}
