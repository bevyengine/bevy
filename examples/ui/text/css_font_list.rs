//! This example demonstrates selecting fonts from CSS font-family lists.

use bevy::prelude::*;

const FONT_ASSETS: &[&str] = &[
    "fonts/FiraSans-Bold.ttf",
    "fonts/FiraMono-Medium.ttf",
    "fonts/MonaSans-VariableFont.ttf",
    "fonts/EBGaramond12-Regular.otf",
];

#[derive(Resource)]
struct LoadedFontAssets {
    _handles: Vec<Handle<Font>>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.insert_resource(LoadedFontAssets {
        _handles: FONT_ASSETS
            .iter()
            .map(|font_asset| asset_server.load(*font_asset))
            .collect(),
    });
    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            row_gap: px(25),
            ..default()
        },
        children![
            (
                Text::new("CSS Font Lists"),
                TextFont::from_font_size(FontSize::Px(32.)),
                Underline,
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(20),
                    padding: px(16).left(),
                    ..default()
                },
                Children::spawn(SpawnIter(
                    (0..FONT_ASSETS.len())
                        .map(|start| {
                            FONT_ASSETS
                                .iter()
                                .cycle()
                                .skip(start)
                                .take(FONT_ASSETS.len())
                                .map(|font_asset| format!("{font_asset}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .map(|list| {
                            (
                                Text::new(list.replace(", ", "\n")),
                                TextFont {
                                    font: FontSource::css(list),
                                    font_size: FontSize::Px(16.),
                                    ..default()
                                },
                                Node {
                                    padding: px(4.).all(),
                                    ..default()
                                },
                                TextLayout::no_wrap(),
                                Outline::default(),
                            )
                        })
                )),
            ),
        ],
    ));
}
