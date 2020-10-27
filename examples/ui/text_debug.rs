use bevy::prelude::*;
extern crate rand;

/// This example is for debugging text layout
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(infotext_system.system())
        .add_system(change_text_system.system())
        .run();
}

struct TextChanges;

fn infotext_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    left: Val::Px(15.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text {
                value: "This is\ntext with\nline breaks\nin the top left".to_string(),
                font: font.clone(),
                style: TextStyle {
                    font_size: 50.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        });
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..Default::default()
                },
                size: Size {
                    width: Val::Px(400.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text {
                value:
                    "This is very long text with limited width in the top right and is also pink"
                        .to_string(),
                font: font.clone(),
                style: TextStyle {
                    font_size: 50.0,
                    color: Color::rgb(0.8, 0.2, 0.7),
                },
            },
            ..Default::default()
        });
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text {
                value: "This text changes in the bottom right".to_string(),
                font: font.clone(),
                style: TextStyle {
                    font_size: 50.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        })
        .with(TextChanges);
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Px(5.0),
                    left: Val::Px(15.0),
                    ..Default::default()
                },
                size: Size {
                    width: Val::Px(200.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text {
                value: "This\ntext has\nline breaks and also a set width in the bottom left"
                    .to_string(),
                font: font.clone(),
                style: TextStyle {
                    font_size: 50.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        });
}

fn change_text_system(mut text: Mut<Text>, _: &TextChanges) {
    text.value = format!(
        "This text changes in the bottom right {}",
        rand::random::<u16>(),
    );
}
