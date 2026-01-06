//! This example displays a scrollable list of all available system fonts.
//! Demonstrates loading and querying system fonts via cosmic-text.

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, input::mouse::MouseScrollUnit, prelude::*,
    text::CosmicFontSystem,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, setup);

    app.world_mut()
        .resource_mut::<CosmicFontSystem>()
        .db_mut()
        .load_system_fonts();

    app.run();
}

fn setup(mut commands: Commands, font_system: Res<CosmicFontSystem>) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                row_gap: px(10.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .with_children(|builder| {
            builder.spawn(Text::new(format!(
                "Total fonts available: {}",
                font_system.db().len(),
            )));

            builder
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    overflow: Overflow::scroll_y(),
                    align_items: AlignItems::Stretch,
                    ..default()
                })
                .with_children(|builder| {
                    let mut families: Vec<(String, String)> = Vec::new();
                    for face in font_system.db().faces() {
                        for (name, lang) in &face.families {
                            families.push((name.to_string(), lang.to_string()));
                        }
                    }
                    families.sort_unstable();
                    families.dedup();
                    for (family, language) in families {
                        builder.spawn((
                            Node {
                                display: Display::Grid,
                                grid_template_columns: vec![
                                    GridTrack::flex(1.),
                                    GridTrack::flex(1.),
                                ],
                                padding: px(6).all(),
                                column_gap: px(50.),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                            children![
                                (
                                    Text::new(&family),
                                    TextFont {
                                        font: FontSource::Family(family.into()),
                                        ..default()
                                    },
                                    TextLayout::new_with_no_wrap()
                                ),
                                (
                                    Text::new(language),
                                    TextLayout::new_with_no_wrap(),
                                    Node {
                                        justify_self: JustifySelf::End,
                                        ..default()
                                    }
                                )
                            ],
                        ));
                    }
                })
                .observe(
                    |on_scroll: On<Pointer<Scroll>>,
                     mut query: Query<(&mut ScrollPosition, &ComputedNode)>| {
                        if let Ok((mut scroll_position, node)) = query.get_mut(on_scroll.entity) {
                            let dy = match on_scroll.unit {
                                MouseScrollUnit::Line => on_scroll.y * 20.,
                                MouseScrollUnit::Pixel => on_scroll.y,
                            };
                            let range = (node.content_size.y - node.size.y).max(0.)
                                * node.inverse_scale_factor;
                            scroll_position.y = (scroll_position.y - dy).clamp(0., range);
                        }
                    },
                );
        });
}
