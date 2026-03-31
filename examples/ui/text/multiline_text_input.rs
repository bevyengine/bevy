//! Demonstrates a single, minimal multiline [`EditableText`] widget.

use bevy::color::palettes::css::{DARK_SLATE_GRAY, YELLOW};
use bevy::input_focus::{AutoFocus, InputDispatchPlugin};
use bevy::prelude::*;
use bevy::text::{EditableText, FontCx, LayoutCx, TextCursorStyle};
use bevy::ui_widgets::EditableTextInputPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            EditableTextInputPlugin,
            // Required so keyboard input is sent to the focused `EditableText`.
            InputDispatchPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, report_bounds)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: px(300.),
                    border: px(2.).all(),
                    padding: px(8.).all(),
                    ..default()
                },
                EditableText {
                    visible_lines: Some(8.),
                    ..default()
                },
                TextLayout {
                    linebreak: LineBreak::AnyCharacter,
                    ..default()
                },
                TextFont {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                    font_size: FontSize::Px(30.),
                    ..default()
                },
                TextCursorStyle::default(),
                BackgroundColor(DARK_SLATE_GRAY.into()),
                BorderColor::all(YELLOW),
                AutoFocus,
            ));
        });
}

fn report_bounds(
    mut font_cx: ResMut<FontCx>,
    mut layout_cx: ResMut<LayoutCx>,
    mut query: Query<(&mut EditableText, &ComputedNode), Changed<EditableText>>,
) {
    for (mut editable_text, node) in query.iter_mut() {
        let mut driver = editable_text
            .bypass_change_detection()
            .editor
            .driver(&mut font_cx.0, &mut layout_cx.0);
        let w = driver.layout().full_width();
        println!("node width = {:?}", node.size.x);
        println!("layout width = {:?}", w);
    }
}
