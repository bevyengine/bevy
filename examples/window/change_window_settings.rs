use bevy::prelude::*;

/// This example illustrates how to change the window settings from a system
fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "I am a window!".to_string(),
            width: 500,
            height: 300,
            vsync: true,
            resizable: false,
            ..Default::default()
        })
        .add_default_plugins()
        .add_system(change_title.system())
        .run();
}

fn change_title(time: Res<Time>, mut windows: ResMut<Windows>) {
    if (time.delta_seconds * 1000.) as i32 % 10 == 1 {
        windows
            .get_primary_mut()
            .unwrap()
            .set_title(format!("{}", time.delta_seconds));
    }
}
