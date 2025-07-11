//! This example tests scale factor and scrolling

use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::RED;
use bevy::prelude::*;

#[derive(Component)]
struct DragNode;

#[derive(Component)]
struct ScrollableNode;

#[derive(Component)]
struct Tile(usize);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct ScrollStart(Vec2);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.insert_resource(UiScale(0.5));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
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
                    scroll_position.0 = start.0 - drag.distance / ui_scale.0;
                }
            },
        )
        .observe(
            |
            _: On<Pointer<DragStart>>,
             mut scroll_position_query: Query<(
                &ScrollPosition,
                &mut ScrollStart),
                With<ScrollableNode>,
            >| {
                if let Ok((scroll_pos,mut start)) = scroll_position_query.single_mut() {
                    start.0 = scroll_pos.0;
                }
            },
        )
        .with_children(|commands| {
            for x in 0..50 {
                commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        for y in 0..50 {
                            let color = if (x + y) % 2 == 1 {
                                NAVY.into()
                            } else {
                                Color::BLACK
                            };
                            commands
                                .spawn((
                                    Node {
                                        width: Val::Px(100.),
                                        height: Val::Px(100.),
                                        min_height: Val::Px(100.),
                                        ..Default::default()
                                    },
                                    Pickable {
                                        should_block_lower: false,
                                        is_hoverable: true,
                                    },
                                    Tile((x + y) % 2),
                                    BackgroundColor(color),
                                ))
                                .observe(|on_enter: On<Pointer<Over>>, mut query: Query<&mut BackgroundColor>, | {
                                    if let Ok(mut background_color) = query.get_mut(on_enter.target()) {
                                        background_color.0 = RED.into();
                                    }
                                })
                                .observe(|on_enter: On<Pointer<Out>>, mut query: Query<(&mut BackgroundColor, &Tile)>,| {
                                    if let Ok((mut background_color, tile)) = query.get_mut(on_enter.target()) {
                                        background_color.0 =
                                            if tile.0 == 1 {
                                                NAVY.into()
                                            } else {
                                                Color::BLACK
                                            };
                                    }
                                })
                                ;
                        }
                    });
            }
        });
}
