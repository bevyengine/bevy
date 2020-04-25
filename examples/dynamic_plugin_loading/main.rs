use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .load_plugin(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/target/debug/libexample_plugin.so"
        ))
        .run();
}
