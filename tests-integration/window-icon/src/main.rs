use bevy::{prelude::*, window::WindowIcon};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Window Icon Example".into(),
                ..default()
            }),
            primary_window_icon: Some({
                #[cfg(target_os = "windows")]
                {
                    WindowIcon::PlatformSpecific(bevy::window::WindowIconSource::ResourceName(
                        "aaa_my_icon".to_string(),
                    ))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    WindowIcon::PlatformDefault
                }
            }),
            ..default()
        }))
        .run();
}
