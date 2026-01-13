//! This example demonstrates generic font families

use bevy::{prelude::*, text::CosmicFontSystem};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins).add_systems(Startup, setup);

    app.world_mut()
        .resource_mut::<CosmicFontSystem>()
        .db_mut()
        .load_system_fonts();

    app.run();
}

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            left: px(250),
            top: px(250),
            row_gap: px(10.),
            ..Default::default()
        },
        children![
            (
                Text::new("generic sans serif font"),
                TextFont::from(FontSource::SansSerif).with_font_size(30.)
            ),
            (
                Text::new("generic serif font"),
                TextFont::from(FontSource::Serif).with_font_size(30.)
            ),
            (
                Text::new("generic fantasy font"),
                TextFont::from(FontSource::Fantasy).with_font_size(30.)
            ),
            (
                Text::new("generic cursive font"),
                TextFont::from(FontSource::Cursive).with_font_size(30.)
            ),
            (
                Text::new("generic monospace font"),
                TextFont::from(FontSource::Monospace).with_font_size(30.)
            )
        ],
    ));
}
