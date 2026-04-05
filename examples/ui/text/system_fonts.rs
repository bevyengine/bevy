//! This example displays a scrollable list of all available system fonts.
//! Demonstrates querying system fonts via `FontCx`.

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, input::mouse::MouseScrollUnit, prelude::*, text::FontCx,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, setup);

    app.run();
}

fn setup(mut commands: Commands, mut font_system: ResMut<FontCx>) {
    let mut families: Vec<String> = font_system
        .0
        .collection
        .family_names()
        .map(ToOwned::to_owned)
        .collect();
    families.sort_unstable();
    families.dedup();
    let family_count = families.len();

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
        .with_children(move |builder| {
            builder.spawn(Text::new(format!(
                "Total available fonts: {}",
                family_count,
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
                    for family in families {
                        let font = FontSource::Family(family.clone().into());
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
                                    TextFont { font, ..default() },
                                    TextLayout::new_with_no_wrap()
                                ),
                                (Text::new(family), TextLayout::new_with_no_wrap()),
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
