use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(dropped_file_system.system())
        .run();
}

fn dropped_file_system(
    mut reader: Local<EventReader<FileDragAndDrop>>,
    events: Res<Events<FileDragAndDrop>>,
) {
    for event in reader.iter(&events) {
        println!("{:?}", event);
    }
}
