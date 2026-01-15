//! This example demonstrates generic font families,
//! which look up a matching font from installed fonts on the user's device.
//!
//! This is used as a fallback in case a specific selected font is available,
//! ensuring that the provided font approximately matches the needs
//! of a given piece of text.
//!
//! This feature is most useful for non-game applications;
//! most games instead choose to simply bundle their required fonts
//! to ensure a unified visual look.

use bevy::{
    color::palettes::{
        css::{WHEAT, YELLOW},
        tailwind::ZINC_600,
    },
    prelude::*,
    text::CosmicFontSystem,
};

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

const FONT_SIZE: f32 = 25.;

fn setup(mut commands: Commands, font_system: Res<CosmicFontSystem>) {
    // UI camera
    commands.spawn(Camera2d);

    commands
        .spawn((Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::fr(3, 1.)],
            margin: UiRect::AUTO,
            row_gap: px(25),
            column_gap: px(15),
            ..Default::default()
        },))
        .with_children(|builder| {
            builder.spawn((
                Node {
                    justify_self: JustifySelf::Center,
                    grid_column: GridPlacement::span(3),
                    margin: UiRect::bottom(px(15)),
                    ..default()
                },
                Text::new("Generic Font Families"),
                TextFont::from_font_size(FONT_SIZE),
                Underline,
            ));

            let outline = Outline {
                color: ZINC_600.into(),
                width: px(2.),
                offset: px(4.),
            };

            for (source, description) in [
                (FontSource::SansSerif, "generic sans serif font"),
                (FontSource::Serif, "generic serif font"),
                (FontSource::Fantasy, "generic fantasy font"),
                (FontSource::Cursive, "generic cursive font"),
                (FontSource::Monospace, "generic monospace font"),
            ] {
                builder.spawn((
                    Text::new(description),
                    TextFont::from(source.clone()).with_font_size(FONT_SIZE),
                    TextColor(WHEAT.into()),
                    TextLayout::new_with_justify(Justify::Center),
                    outline,
                ));

                builder.spawn((
                    Text::new(format!("FontSource::{source:?}")),
                    TextFont::from_font_size(FONT_SIZE),
                    TextColor(YELLOW.into()),
                    TextLayout::new_with_justify(Justify::Center),
                    outline,
                ));

                // Get the family name for the `FontSource` from `CosmicFontSystem`.
                // The unwrap here is safe, `get_family_name` only returns `None` if the source is a handle.
                let family_name = font_system.get_family(&source).unwrap();
                builder.spawn((
                    Text::new(family_name),
                    TextFont::from_font_size(FONT_SIZE),
                    TextLayout::new_with_justify(Justify::Center),
                    outline,
                ));
            }
        });
}
