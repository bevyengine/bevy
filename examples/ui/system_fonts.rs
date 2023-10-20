//! This example demonstrates using system fonts, which are already installed on the user's computer.
//!
//! System fonts won't always be present: if they're not found, a fallback will be used instead.
//!
//! If you need consistent font rendering for aesthetic reasons, you should package and ship your own font as an asset instead.

use bevy::{
    prelude::*,
    text::{FontQuery, TextPipeline},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut text_pipeline: ResMut<TextPipeline>) {
    text_pipeline.load_system_fonts();
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font_size: 42.0,
        color: Color::WHITE,
        ..default()
    };
    let mut sections = vec![];

    // the default font is sans-serif
    sections.push(TextSection {
        value: "(The default font)\n".to_string(),
        style: TextStyle {
            font: FontQuery::default().into(),
            ..text_style
        },
    });

    // sans-serif
    sections.push(TextSection {
        value: "sans-serif\n".to_string(),
        style: TextStyle {
            font: FontQuery::sans_serif().into(),
            ..text_style
        },
    });

    // serif
    sections.push(TextSection {
        value: "serif\n".to_string(),
        style: TextStyle {
            font: FontQuery::serif().into(),
            ..text_style
        },
    });

    // fantasy
    sections.push(TextSection {
        value: "fantasy\n".to_string(),
        style: TextStyle {
            font: FontQuery::fantasy().into(),
            ..text_style
        },
    });

    // cursive
    sections.push(TextSection {
        value: "cursive\n".to_string(),
        style: TextStyle {
            font: FontQuery::cursive().into(),
            ..text_style
        },
    });

    // monospace
    sections.push(TextSection {
        value: "monospace\n".to_string(),
        style: TextStyle {
            font: FontQuery::monospace().into(),
            ..text_style
        },
    });

    // you can also refer to families by name
    for family in [
        "Arial",
        "Comic Sans MS",
        "Impact",
        "Courier New",
        "Times New Roman",
        "(A fallback when not found)",
    ] {
        sections.push(TextSection {
            value: family.to_string() + "\n",
            style: TextStyle {
                font: FontQuery::family(family).into(),
                ..text_style
            },
        })
    }

    // bidirectional text
    sections.push(TextSection {
        value: "We can even render اللغة العربية and\n".to_string(),
        style: TextStyle {
            font: FontQuery::serif().into(),
            ..text_style
        },
    });

    // and emojis
    sections.push(TextSection {
        value: "emojis: 🐣🐤🐥🐔🐓🦃🐦🐧🕊️🦅🦆🦢🦉🦩🦚🦜\n".to_string(),
        style: TextStyle {
            font: FontQuery::cursive().into(),
            ..text_style
        },
    });

    commands.spawn(TextBundle::from_sections(sections));
}
