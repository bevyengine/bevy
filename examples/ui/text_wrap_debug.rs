//! This example demonstrates text wrapping and use of the `LineBreakOn` property.

use argh::FromArgs;
use bevy::{prelude::*, text::LineBreak, window::WindowResolution, winit::WinitSettings};

#[derive(FromArgs, Resource)]
/// `text_wrap_debug` demonstrates text wrapping and use of the `LineBreakOn` property
struct Args {
    #[argh(option)]
    /// window scale factor
    scale_factor: Option<f32>,

    #[argh(option, default = "1.")]
    /// ui scale factor
    ui_scale: f32,
}

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    let window = if let Some(scale_factor) = args.scale_factor {
        Window {
            resolution: WindowResolution::default().with_scale_factor_override(scale_factor),
            ..Default::default()
        }
    } else {
        Window::default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..Default::default()
        }))
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(UiScale(args.ui_scale))
        .add_systems(Startup, spawn)
        .run();
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 12.0,
        ..default()
    };

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background_color: Color::BLACK.into(),
            ..Default::default()
        })
        .id();

    for linebreak in [
        LineBreak::AnyCharacter,
        LineBreak::WordBoundary,
        LineBreak::WordOrCharacter,
        LineBreak::NoWrap,
    ] {
        let row_id = commands
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceAround,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.),
                    height: Val::Percent(50.),
                    ..Default::default()
                },
                ..Default::default()
            })
            .id();

        let justifications = vec![
            JustifyContent::Center,
            JustifyContent::FlexStart,
            JustifyContent::FlexEnd,
            JustifyContent::SpaceAround,
            JustifyContent::SpaceBetween,
            JustifyContent::SpaceEvenly,
        ];

        for (i, justification) in justifications.into_iter().enumerate() {
            let c = 0.3 + i as f32 * 0.1;
            let column_id = commands
                .spawn(NodeBundle {
                    style: Style {
                        justify_content: justification,
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(16.),
                        height: Val::Percent(95.),
                        overflow: Overflow::clip_x(),
                        ..Default::default()
                    },
                    background_color: Color::srgb(0.5, c, 1.0 - c).into(),
                    ..Default::default()
                })
                .id();

            let messages = [
                format!("JustifyContent::{justification:?}"),
                format!("LineBreakOn::{linebreak:?}"),
                "Line 1\nLine 2".to_string(),
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Maecenas auctor, nunc ac faucibus fringilla.".to_string(),
                "pneumonoultramicroscopicsilicovolcanoconiosis".to_string()
            ];

            for (j, message) in messages.into_iter().enumerate() {
                commands.entity(column_id).with_child((
                    TextNEW(message.clone()),
                    text_style.clone(),
                    TextBlock::new_with_justify(JustifyText::Left).with_linebreak(linebreak),
                    BackgroundColor(Color::srgb(0.8 - j as f32 * 0.2, 0., 0.).into()),
                ));
            }
            commands.entity(row_id).add_child(column_id);
        }
        commands.entity(root).add_child(row_id);
    }
}
