//! Virtual keyboard example

use bevy::{
    color::palettes::css::{NAVY, WHITE},
    feathers::{
        controls::{VirtualKeyPressed, VirtualKeyboard},
        dark_theme::create_dark_theme,
        theme::UiTheme,
        FeathersPlugins,
    },
    input_focus::{
        tab_navigation::{TabGroup, TabIndex},
        AutoFocus, InputFocus,
    },
    prelude::*,
    text::{EditableText, TextCursorStyle, TextEdit},
    ui_widgets::TextInput,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn())
        .run();
}

fn on_virtual_key_pressed(
    virtual_key_pressed: On<VirtualKeyPressed<&'static str>>,
    mut query: Query<(Entity, &mut EditableText)>,
    mut input_focus: ResMut<InputFocus>,
) {
    println!("Virtual keyboard key pressed: {}", virtual_key_pressed.key);
    let Ok((entity_id, mut text)) = query.single_mut() else {
        return;
    };
    text.queue_edit(match virtual_key_pressed.key {
        "space" => TextEdit::Insert(" ".into()),
        "enter" => TextEdit::Insert("\n".into()),
        "backspace" => TextEdit::Backspace,
        "left" => TextEdit::Left(false),
        "right" => TextEdit::Right(false),
        "up" => TextEdit::Up(false),
        "down" => TextEdit::Down(false),
        "home" => TextEdit::LineStart(false),
        "end" => TextEdit::LineEnd(false),
        key if key.len() == 1 => TextEdit::Insert(key.into()),
        _ => return,
    });
    input_focus.set(entity_id, bevy::input_focus::FocusCause::Navigated);
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, text_input(), keyboard()]
}

fn keyboard() -> impl Scene {
    let keys = [
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", ".", ","],
        vec!["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
        vec!["A", "S", "D", "F", "G", "H", "J", "K", "L", "'"],
        vec!["Z", "X", "C", "V", "B", "N", "M", "-", "/"],
        vec!["space", "enter", "backspace"],
        vec!["left", "right", "up", "down", "home", "end"],
    ];

    bsn! {
        Node {
            width: percent(100),
            bottom: px(0),
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::End,
        }
        Children [(
            Node {
                flex_direction: FlexDirection::Column,
                border: px(5),
                row_gap: px(5),
                padding: px(5),
                align_items: AlignItems::Center,
                margin: px(25),
                border_radius: BorderRadius::all(px(10)),
            }
            BackgroundColor(NAVY)
            BorderColor::all(Color::WHITE)
            Children [
                Text("virtual keyboard"),
                (
                    @VirtualKeyboard::<&str> { @keys: keys }
                    on(on_virtual_key_pressed)
                )
            ]
        )]
    }
}

fn text_input() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            padding: px(25),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        TabGroup
        Children [(
            Node {
                width: percent(80),
                border: px(5),
                padding: px(5),
                flex_grow: 0.0,
                border_radius: BorderRadius::all(px(10)),
            }
            BorderColor::from(WHITE)
            TextInput
            EditableText {
                visible_lines: { Some(5.) }
                allow_newlines: true,
            }
            TextLayout::no_wrap()
            TextCursorStyle {
                color: WHITE
            }
            TextFont {
                font_size: FontSize::Px(25.),
            }
            TabIndex(0)
            AutoFocus
        )]
    }
}
