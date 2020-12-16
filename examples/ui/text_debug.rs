use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

/// This example is for debugging text layout
fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_startup_system(infotext_system.system())
        .add_system(change_text_system.system())
        .run();
}

struct TextChanges;

fn infotext_system(commands: &mut Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(CameraUiBundle::default());
    commands.spawn(TextBundle {
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
                alignment: TextAlignment::default(),
            },
        },
        ..Default::default()
    });
    commands.spawn(TextBundle {
        style: Style {
            align_self: AlignSelf::FlexEnd,
            position_type: PositionType::Absolute,
            position: Rect {
                top: Val::Px(5.0),
                right: Val::Px(15.0),
                ..Default::default()
            },
            max_size: Size {
                width: Val::Px(400.),
                height: Val::Undefined,
            },
            ..Default::default()
        },
        text: Text {
            value: "This is very long text with limited width in the top right and is also pink"
                .to_string(),
            font: font.clone(),
            style: TextStyle {
                font_size: 50.0,
                color: Color::rgb(0.8, 0.2, 0.7),
                alignment: TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    vertical: VerticalAlign::Center,
                },
            },
        },
        ..Default::default()
    });
    commands
        .spawn(TextBundle {
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
                    font_size: 30.0,
                    color: Color::WHITE,
                    alignment: TextAlignment::default(),
                },
            },
            ..Default::default()
        })
        .with(TextChanges);
    commands.spawn(TextBundle {
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
            font,
            style: TextStyle {
                font_size: 50.0,
                color: Color::WHITE,
                alignment: TextAlignment::default(),
            },
        },
        ..Default::default()
    });
}

fn change_text_system(
    time: Res<Time>,
    diagnostics: Res<Diagnostics>,
    mut query: Query<&mut Text, With<TextChanges>>,
) {
    for mut text in query.iter_mut() {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_avg) = fps_diagnostic.average() {
                fps = fps_avg;
            }
        }

        let mut frame_time = time.delta_seconds_f64();
        if let Some(frame_time_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        {
            if let Some(frame_time_avg) = frame_time_diagnostic.average() {
                frame_time = frame_time_avg;
            }
        }

        text.value = format!(
            "This text changes in the bottom right - {:.1} fps, {:.3} ms/frame",
            fps,
            frame_time * 1000.0,
        );
    }
}
