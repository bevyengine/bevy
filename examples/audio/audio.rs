//! This example illustrates how to load and play an audio file.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_info_text)
        .add_system(play_on_space)
        .run();
}

fn setup_info_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font,
        font_size: 60.0,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment::CENTER;
    // 2d camera
    commands.spawn(Camera2dBundle::default());
    // Demonstrate changing translation
    commands.spawn(Text2dBundle {
        text: Text::from_section("Press SPACE to play", text_style.clone()),
        ..default()
    });
}
    
fn play_on_space(asset_server: Res<AssetServer>, audio: Res<Audio>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        let music = asset_server.load("sounds/Windless Slopes.ogg");
        audio.play(music);
    }
}
