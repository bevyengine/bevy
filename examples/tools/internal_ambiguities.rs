//! Checks that a schedule with all default plugins runs, and no internal system execution order ambiguities exist.
//!
//! Note that execution order ambiguities can and are deliberately ignored.
//! If any of these are causing issues to the deterministic execution of your game, please open an issue!
//!
//! This is mostly used for testing that Bevy works as expected, both on your device and on CI.
//! Consider it an advanced "hello world". You should see an empty window open.

use bevy::{ecs::schedule::ReportExecutionOrderAmbiguities, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ReportExecutionOrderAmbiguities)
        .run();
}
