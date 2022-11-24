//! This example demonstrates how text is displayed with each TextAlignment

use bevy::prelude::*;

const ALIGNMENTS: [TextAlignment; 9] = [
    TextAlignment::TOP_LEFT,
    TextAlignment::TOP_CENTER,
    TextAlignment::TOP_RIGHT,
    TextAlignment::CENTER_LEFT,
    TextAlignment::CENTER,
    TextAlignment::CENTER_RIGHT,
    TextAlignment::BOTTOM_LEFT,
    TextAlignment::BOTTOM_CENTER,
    TextAlignment::BOTTOM_RIGHT,
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update)
        .run();
}
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 24.0,
        color: Color::WHITE,
    };

    commands.insert_resource(UiScale { scale: 0.5 });
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                size: Size::new(Val::Percent(100.), Val::Percent(100.)),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(80.), Val::Percent(80.)),
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::NAVY),
                    ..Default::default()
                })
                .with_children(|builder| {
                    builder.spawn(TextBundle {
                        text: Text::from_section("".to_string(), text_style.clone()),
                        ..Default::default()
                    });
                });
        });
}

pub fn update(
    mut t: Local<f32>,
    mut i: Local<usize>,
    mut uiscale: ResMut<UiScale>,
    time: Res<Time>,
    mut text_query: Query<&mut Text>,
) {
    *t -= time.delta_seconds();
    if *t <= 0. {
        *t = 0.5;
        *i = (*i + 1) % ALIGNMENTS.len();
        let mut text = text_query.single_mut();
        text.alignment = ALIGNMENTS[*i];
        text.sections[0].value = format!(
            "{:?}-{:?}",
            text.alignment.vertical, text.alignment.horizontal
        );

        if *i % 2 == 0 {
            uiscale.scale += 0.5;
            if uiscale.scale > 5.0 {
                uiscale.scale = 0.5;
            }
        }
    }
}
