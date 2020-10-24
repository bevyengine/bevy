use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_resource(ClearColor(Color::BLACK)) // the window's background colour
        .add_startup_system(setup.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            text: Text {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                value: "This\ntext\nwraps".to_string(), // this is the text I want to position in the top right
                style: TextStyle {
                    color: Color::rgba(1.0, 1.0, 1.0, 0.5), // White text
                    font_size: 40.0,
                },
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    right: Val::Px(5.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .spawn(TextComponents {
            text: Text {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                value: "Thistextwraps".to_string(), // this is a "shadow" that demonstrates what's happening
                style: TextStyle {
                    color: Color::rgba(0.7, 0.7, 1.0, 0.5), // Blue text
                    font_size: 40.0,
                },
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    right: Val::Px(5.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });
}
