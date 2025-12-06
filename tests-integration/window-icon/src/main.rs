use bevy::{
    prelude::*,
    window::{WindowIcon, WindowIconSource},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Window Icon Example".into(),
                ..default()
            }),
            primary_window_icon: Some(WindowIcon::PlatformSpecific(
                WindowIconSource::ResourceName("aaa_my_icon".to_string()),
            )),
            ..default()
        }))
        .run();
}
