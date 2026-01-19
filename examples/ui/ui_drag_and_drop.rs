//! Demonstrates dragging and dropping UI nodes

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

const COLUMNS: i16 = 10;
const ROWS: i16 = 10;
const TILE_SIZE: f32 = 40.;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((Node {
            display: Display::Grid,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..Default::default()
        }, Pickable::IGNORE, BackgroundColor(Color::srgb(0.4, 0.4, 0.4))))
        .with_children(|parent| {
            let tile_colors = [
                Color::srgb(0.2, 0.2, 0.8),
                Color::srgb(0.8, 0.2, 0.2)
            ];
            for column in 0..COLUMNS {
                for row in 0..ROWS {
                    let i = column + row * COLUMNS;
                    let tile_color = tile_colors[((row % 2) + column) as usize % tile_colors.len()];
                    let tile_border_color = tile_color.darker(0.025);
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(TILE_SIZE),
                                height: Val::Px(TILE_SIZE),
                                border: UiRect::all(Val::Px(4.)),
                                grid_row: GridPlacement::start(row + 1),
                                grid_column: GridPlacement::start(column + 1),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..Default::default()
                            },
                            BorderColor::all(tile_border_color),
                            BackgroundColor(tile_color),
                            Outline {
                                width: Val::Px(2.),
                                offset: Val::ZERO,
                                color: Color::NONE,
                            },
                            Pickable {
                                should_block_lower: false,
                                is_hoverable: true,
                            },
                            GlobalZIndex::default()
                        ))
                        .observe(move |on_over: On<Pointer<Over>>, mut query: Query<(&mut BackgroundColor, &mut BorderColor)>| {
                            if let Ok((mut background_color, mut border_color)) = query.get_mut(on_over.event_target()) {
                                background_color.0 = tile_color.lighter(0.1);
                                border_color.set_all(tile_border_color.lighter(0.1));
                            }
                        })
                        .observe(move |on_out: On<Pointer<Out>>, mut query: Query<(&mut BackgroundColor, &mut BorderColor)>| {
                            if let Ok((mut background_color, mut border_color)) = query.get_mut(on_out.event_target()) {
                                background_color.0 = tile_color;
                                border_color.set_all(tile_border_color);
                            }
                        })
                        .observe(|on_drag_start: On<Pointer<DragStart>>, mut query: Query<(&mut Outline, &mut GlobalZIndex)>| {
                            if let Ok((mut outline, mut global_zindex, )) = query.get_mut(on_drag_start.event_target()) {
                                outline.color = Color::WHITE;
                                global_zindex.0 = 1;
                            }
                        })
                        .observe(|on_drag: On<Pointer<Drag>>, mut query: Query<&mut UiTransform>| {
                            if let Ok(mut transform) = query.get_mut(on_drag.event_target()) {
                                transform.translation = Val2::px(on_drag.distance.x, on_drag.distance.y);
                            }
                        })
                        .observe(move |on_drag_end: On<Pointer<DragEnd>>, mut query: Query<(&mut UiTransform, &mut Outline, &mut GlobalZIndex)>| {
                            if let Ok((mut transform, mut outline, mut global_zindex)) = query.get_mut(on_drag_end.event_target()) {
                                transform.translation = Val2::ZERO;
                                outline.color = Color::NONE;
                                global_zindex.0 = 0;
                            }
                        })
                        .observe(|on_drag_drop: On<Pointer<DragDrop>>, mut query: Query<&mut Node>| {
                            if let Ok([mut a, mut b]) = query.get_many_mut([on_drag_drop.event_target(), on_drag_drop.dropped]) {
                                core::mem::swap(&mut a.grid_row, &mut b.grid_row);
                                core::mem::swap(&mut a.grid_column, &mut b.grid_column);
                            }
                        })
                        .with_child((Text::new(format!("{i}")), Pickable::IGNORE));
                }
            }
        });
}
