use bevy::prelude::*;
use bevy::winit::ChangeWindow;

/// This example illustrates how to customize the default window settings
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

fn change_title(
    time: Res<Time>,
    windows: Res<Windows>,
    mut change_window_events: ResMut<Events<ChangeWindow>>,
) {
    if (time.delta_seconds * 1000.) as i32 % 10 == 1 {
        let id = windows.get_primary().unwrap().id;

        change_window_events.send(ChangeWindow::SetTitle {
            id,
            title: format!("{}", time.delta_seconds),
        });
    }
}
