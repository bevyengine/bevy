//! This example tests scale factor, dragging and scrolling

use bevy::color::palettes::css::RED;
use bevy::prelude::*;

#[derive(Component)]
struct ScrollableNode;

#[derive(Component)]
struct TileColor(Color);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct ScrollStart(Vec2);

fn setup(mut commands: Commands) {
    let w = 60;
    let h = 40;

    commands.spawn(Camera2d);
    commands.insert_resource(UiScale(0.5));

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                overflow: Overflow::scroll(),
                ..Default::default()
            },
            ScrollPosition(Vec2::ZERO),
            ScrollableNode,
            ScrollStart(Vec2::ZERO),
        ))
        .observe(
            |
            drag: On<Pointer<Drag>>,
             ui_scale: Res<UiScale>,
             mut scroll_position_query: Query<(
                &mut ScrollPosition,
                &ScrollStart),
                With<ScrollableNode>,
             >| {
                if let Ok((mut scroll_position, start)) = scroll_position_query.single_mut() {
                    scroll_position.0 = (start.0 - drag.distance / ui_scale.0).max(Vec2::ZERO);
                }
            },
        )
        .observe(
            |
            on: On<Pointer<DragStart>>,
             mut scroll_position_query: Query<(
                &ComputedNode,
                &mut ScrollStart),
                With<ScrollableNode>,
            >| {
                if on.entity() != on.original_entity() {
                    return;
                }
                if let Ok((computed_node, mut start)) = scroll_position_query.single_mut() {
                    start.0 = computed_node.scroll_position * computed_node.inverse_scale_factor;
                }
            },
        )

        .with_children(|commands| {
            commands
            .spawn(Node {
                display: Display::Grid,
                grid_template_rows: RepeatedGridTrack::px(w as i32, 100.),
                grid_template_columns: RepeatedGridTrack::px(h as i32, 100.),
                ..Default::default()
            })
            .with_children(|commands| {
                for y in 0..h {
                    for x in 0..w {
                        let tile_color = if (x + y) % 2 == 1 {
                            let hue = ((x as f32 / w as f32) * 270.0) + ((y as f32 / h as f32) * 90.0);
                            Color::hsl(hue, 1., 0.5)
                        } else {
                            Color::BLACK
                        };
                        commands
                            .spawn((
                                Node {
                                    grid_row: GridPlacement::start(y + 1),
                                    grid_column: GridPlacement::start(x + 1),
                                    ..Default::default()
                                },
                                Pickable {
                                    should_block_lower: false,
                                    is_hoverable: true,
                                },
                                TileColor(tile_color),
                                BackgroundColor(tile_color),
                            ))
                            .observe(|on_enter: On<Pointer<Over>>, mut query: Query<&mut BackgroundColor>, | {
                                if let Ok(mut background_color) = query.get_mut(on_enter.entity()) {
                                    background_color.0 = RED.into();
                                }
                            })
                            .observe(|on_enter: On<Pointer<Out>>, mut query: Query<(&mut BackgroundColor, &TileColor)>,| {
                                if let Ok((mut background_color, tile_color)) = query.get_mut(on_enter.entity()) {
                                    background_color.0 = tile_color.0;
                                }
                            });
                        }
                }
            });
        });
}
