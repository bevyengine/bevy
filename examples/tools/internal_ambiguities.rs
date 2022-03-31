//! Reports any Bevy-internal system-order ambiguities
//! This is primarily useful for engine development

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ExecutionOrderAmbiguities::Forbid)
        .run()
}
