use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .load_plugin(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/plugin_loading/example_plugin/target/release/libexample_plugin.so"
        ))
        .run();
}
