//! Reports execution-order ambiguities involving internal Bevy systems.
//! This is primarily useful for engine development

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ExecutionOrderAmbiguities::WarnInternal)
        .run()
}
