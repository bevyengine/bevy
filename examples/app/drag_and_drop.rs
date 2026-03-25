//! An example that shows how to handle drag and drop of files in an app.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, file_drag_and_drop_system)
        .run();
}

fn file_drag_and_drop_system(mut drag_and_drop_reader: MessageReader<FileDragAndDrop>) {
    for drag_and_drop in drag_and_drop_reader.read() {
        info!("{:?}", drag_and_drop);
    }
}
