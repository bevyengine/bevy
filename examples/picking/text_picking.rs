//! Demo picking text.

use bevy::{
    prelude::*,
    text::TextLayoutInfo,
    ui::{text_picking_backend::TextPointer, RelativeCursorPosition},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup,))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Text::new("hello text picking"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(25.0),
                left: Val::Percent(25.0),
                ..default()
            },
            RelativeCursorPosition::default(),
        ))
        .with_children(|cb| {
            // TODO: find a better text string that shows multibyte adherence within bevy's font
            // subset.
            cb.spawn(TextSpan::new(
                "i'm a new span\n●●●●i'm the same span...\n····",
            ));
        })
        .observe(
            |t: Trigger<TextPointer<Click>>, texts: Query<&TextLayoutInfo>| {
                // Observer to get the `PositionedGlyph` at the `Cursor` position.
                let text = texts
                    .get(t.target())
                    .expect("no TLI? This should be unreachable.");

                let Some(positioned_glyph) = text
                    .glyphs
                    .iter()
                    .find(|g| g.byte_index == t.cursor.index && g.line_index == t.cursor.line)
                else {
                    return;
                };

                info!("found positioned glyph from cursor {:?}", positioned_glyph);

                // TODO: Visualize a cursor on click.
            },
        );
}
