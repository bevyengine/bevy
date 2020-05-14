use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .load_plugin("target/debug/libexample_plugin.so")
        .run();
}
