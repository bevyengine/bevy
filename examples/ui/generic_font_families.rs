//! This example demonstrates generic font families

use bevy::{color::palettes::css::YELLOW, prelude::*, text::CosmicFontSystem};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins).add_systems(Startup, setup);

    // The default font will be used where there is no system font matching the
    // generic font variant's font name stored in Cosmic Text's `Database`.
    app.world_mut()
        .resource_mut::<CosmicFontSystem>()
        .db_mut()
        .load_system_fonts();

    app.run();
}

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2d);

    commands
        .spawn((Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::flex(1.0), GridTrack::flex(1.0)],
            margin: UiRect::AUTO,
            row_gap: px(15),
            column_gap: px(30),
            ..Default::default()
        },))
        .with_children(|builder| {
            builder.spawn((
                Node {
                    justify_self: JustifySelf::Center,
                    grid_column: GridPlacement::span(2),
                    margin: UiRect::bottom(px(15)),
                    ..default()
                },
                Text::new("Generic Font Families"),
                TextFont::from_font_size(35.),
                Underline,
            ));

            for (source, description) in [
                (FontSource::SansSerif, "generic sans serif font"),
                (FontSource::Serif, "generic serif font"),
                (FontSource::Fantasy, "generic fantasy font"),
                (FontSource::Cursive, "generic cursive font"),
                (FontSource::Monospace, "generic monospace font"),
            ] {
                builder.spawn((
                    Node {
                        justify_self: JustifySelf::End,
                        ..default()
                    },
                    Text::new(format!("FontSource::{source:?}")),
                    TextFont::from_font_size(40.),
                    TextColor(YELLOW.into()),
                ));

                builder.spawn((
                    Text::new(description),
                    TextFont::from(source).with_font_size(40.0),
                    TextColor::WHITE,
                ));
            }
        });
}
