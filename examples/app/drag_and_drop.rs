//! An example that shows how to handle drag and drop of files in an app.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugin_group(DefaultPlugins)
        .add_system(file_drag_and_drop_system)
        .run();
}

fn file_drag_and_drop_system(mut events: EventReader<FileDragAndDrop>) {
    for event in events.iter() {
        info!("{:?}", event);
    }
}
