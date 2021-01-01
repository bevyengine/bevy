use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(file_drag_and_drop_system.system())
        .run();
}

fn file_drag_and_drop_system(
    mut reader: Local<EventReader<FileDragAndDrop>>,
    events: Res<Events<FileDragAndDrop>>,
) {
    for event in reader.iter(&events) {
        println!("{:?}", event);
    }
}
