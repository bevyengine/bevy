//! Demonstrates a basic player inventory screen

use bevy::{
    color::palettes::css::{DARK_GOLDENROD, GRAY, MAROON},
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .insert_resource(UiScale(2.))
        .run();
}

const COLUMNS: i16 = 7;
const ROWS: i16 = 5;
const TILE_SIZE: f32 = 25.;
const GAP: f32 = 4.;

#[derive(Component, PartialEq)]
enum ItemSlot {
    Head,
    Body,
    Legs,
    Hand,
}

#[derive(Component)]
struct ItemNode;

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let item_border_color = GRAY;

    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(2. * GAP)),
                row_gap: Val::Px(3. * GAP),
                ..Default::default()
            },
            Pickable::IGNORE,
            Outline {
                width: Val::Px(2.),
                color: Color::WHITE,
                ..Default::default()
            },
            BackgroundColor(MAROON.into())
        ))
        .with_children(|parent| {
            parent.spawn(Text::new("Inventory"));

            let left =parent
                .spawn((
                    Node {
                        display: Display::Grid,
                        grid_auto_columns: GridTrack::px(TILE_SIZE),
                        grid_auto_rows: GridTrack::px(TILE_SIZE),
                        row_gap: Val::Px(GAP),
                        ..Default::default()
                    },
                    Pickable::IGNORE,
                )).with_children(|parent| {
                    for (i, (label, item_slot)) in [("head", ItemSlot::Head), ("body", ItemSlot::Body),("legs", ItemSlot::Legs), ("hand", ItemSlot::Hand), ("hand",ItemSlot::Hand)]
                        .into_iter()
                        .enumerate() {

                        parent
                            .spawn((
                                Node {
                                    width: Val::Px(TILE_SIZE),
                                    height: Val::Px(TILE_SIZE),
                                    grid_column: GridPlacement::start(1),
                                    grid_row: GridPlacement::start(i as i16 + 1),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(Val::Px(1.)),

                                    ..Default::default()
                                },
                                BorderColor::all(item_border_color),
                                BackgroundColor(Color::BLACK),
                                item_slot,
                                children![(
                                    Node {
                                        position_type: PositionType::Absolute,
                                        ..Default::default()
                                    },
                                    TextColor(DARK_GOLDENROD.into()),
                                    Text::new(label), TextFont::from_font_size(7.))],
                            ))
                            .observe(
                                move |on_over: On<Pointer<Over>>, mut query: Query<&mut BorderColor>| {
                                    if let Ok(mut border_color) = query.get_mut(on_over.entity()) {
                                        border_color.set_all(item_border_color.lighter(0.5));
                                    }
                                },
                            )
                            .observe(
                                move |on_out: On<Pointer<Out>>, mut query: Query<&mut BorderColor>| {
                                    if let Ok(mut border_color) = query.get_mut(on_out.entity()) {
                                        border_color.set_all(item_border_color);
                                    }
                                },
                            )
                        .with_children(|parent| {
                            parent.spawn((

                                Node {
                                    width: Val::Px(TILE_SIZE),
                                    height: Val::Px(TILE_SIZE),
                                ..Default::default()
                                },
                                Pickable {
                                    should_block_lower: false,
                                    is_hoverable: true,
                                },
                                ItemNode,

                            ))
                            .observe(|on_drag_drop: On<Pointer<DragDrop>>, mut commands: Commands| {
                                let target = on_drag_drop.entity();
                                let dropped = on_drag_drop.dropped;

                                commands.queue(move |world: &mut World| {
                                    if !world.entity(dropped).contains::<ItemNode>() {
                                        return;
                                    }

                                    let pa = world.entity(target).get::<ChildOf>().unwrap().0;
                                    let pb = world.entity(dropped).get::<ChildOf>().unwrap().0;

                                    if let Some(slot_a) = world.entity(pa).get::<ItemSlot>() {
                                        if let Some(slot_b) = world.entity(dropped).get::<ItemSlot>() {
                                            if slot_a != slot_b {
                                                return;
                                            }
                                        }
                                    }

                                    world.entity_mut(target).insert(ChildOf( pb));
                                    world.entity_mut(dropped).insert(ChildOf( pa));

                                });

                            });
                    });
                }
            }).id();

         let right = parent
                .spawn((
        Node {
                display: Display::Grid,
                grid_auto_columns: GridTrack::px(TILE_SIZE),
                grid_auto_rows: GridTrack::px(TILE_SIZE),
                row_gap: Val::Px(GAP),
                column_gap: Val::Px(GAP),
                ..Default::default()
            },
            Pickable::IGNORE,
        )).with_children(|parent| {

            let mut item_list = [
                ("textures/rpg/props/generic-rpg-barrel01.png", ItemSlot::Legs),
                ("textures/rpg/props/generic-rpg-board01.png", ItemSlot::Legs),
                ("textures/rpg/props/generic-rpg-crate01.png", ItemSlot::Body),
                ("textures/rpg/props/generic-rpg-crate03.png", ItemSlot::Body),
                ("textures/rpg/props/generic-rpg-flower01.png", ItemSlot::Head),
                ("textures/rpg/props/generic-rpg-flower02.png", ItemSlot::Head),
                ("textures/rpg/props/generic-rpg-flower03.png", ItemSlot::Head),
                ("textures/rpg/props/generic-rpg-loot01.png", ItemSlot::Hand),
                ("textures/rpg/props/generic-rpg-loot02.png", ItemSlot::Hand),
                ("textures/rpg/props/generic-rpg-loot03.png", ItemSlot::Hand),
                ("textures/rpg/props/generic-rpg-loot04.png", ItemSlot::Hand),
            ]
            .into_iter();

            for row in 1..ROWS + 1 {
                for column in 1..COLUMNS + 1 {
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(TILE_SIZE),
                                height: Val::Px(TILE_SIZE),
                                border: UiRect::all(Val::Px(1.)),
                                grid_row: GridPlacement::start(row),
                                grid_column: GridPlacement::start(column),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..Default::default()
                            },
                            BorderColor::all(item_border_color),
                            BackgroundColor(Color::BLACK),
                        ))
                        .observe(
                            move |on_over: On<Pointer<Over>>,
                                  mut query: Query<&mut BorderColor>| {
                                if let Ok(mut border_color) = query.get_mut(on_over.entity()) {
                                    border_color.set_all(item_border_color.lighter(0.5));
                                }
                            },
                        )
                        .observe(
                            move |on_out: On<Pointer<Out>>, mut query: Query<&mut BorderColor>| {
                                if let Ok(mut border_color) = query.get_mut(on_out.entity()) {
                                    border_color.set_all(item_border_color);
                                }
                            },
                        )
                       .with_children(|parent| {
                            parent.spawn((
                                Node {
                                    width: Val::Px(TILE_SIZE),
                                    height: Val::Px(TILE_SIZE),
                                    border: UiRect::all(Val::Px(1.)),
                                    grid_row: GridPlacement::start(row),
                                    grid_column: GridPlacement::start(column),
                                    ..Default::default()
                                },
                                ZIndex(1),
                                Pickable {
                                    should_block_lower: false,
                                    is_hoverable: true,
                                },
                            )).with_children(|parent| {
                                let mut item_node = parent.spawn((
                                    Node {
                                        width: Val::Px(TILE_SIZE),
                                        height: Val::Px(TILE_SIZE),

                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..Default::default()
                                    },
                                    ZIndex(1),
                                    GlobalZIndex(0),
                                    Pickable {
                                        should_block_lower: false,
                                        is_hoverable: true,
                                    },
                                ));

                                item_node.observe(|on_drag_drop: On<Pointer<DragDrop>>, mut commands: Commands| {
                                let target = on_drag_drop.entity();
                                let dropped = on_drag_drop.dropped;

                                commands.queue(move |world: &mut World| {
                                    if !world.entity(dropped).contains::<ItemNode>() {
                                        return;
                                    }

                                    let pa = world.entity(target).get::<ChildOf>().unwrap().0;
                                    let pb = world.entity(dropped).get::<ChildOf>().unwrap().0;

                                    if let Some(slot_a) = world.entity(pa).get::<ItemSlot>() {
                                        if let Some(slot_b) = world.entity(dropped).get::<ItemSlot>() {
                                            if slot_a != slot_b {
                                                return;
                                            }
                                        }
                                    }

                                    world.entity_mut(target).insert(ChildOf( pb));
                                    world.entity_mut(dropped).insert(ChildOf( pa));
                                });
                            });

                        if let Some((item_image_path, slot)) = item_list.next() {
                            item_node.insert((ItemNode, slot))
                            .observe(|on_drag_start: On<Pointer<DragStart>>, mut query: Query<&mut GlobalZIndex>| {

                                if let Ok(mut global_zindex) = query.get_mut(on_drag_start.entity()) {
                                    global_zindex.0 = 1;
                                }
                            })
                            .observe(|on_drag: On<Pointer<Drag>>, mut query: Query<&mut UiTransform>, ui_scale: Res<UiScale>,| {
                                if let Ok(mut transform) = query.get_mut(on_drag.entity()) {
                                    let drag_distance = on_drag.distance / ui_scale.0;
                                    transform.translation = Val2::px(drag_distance.x, drag_distance.y);
                                }
                            })
                            .observe(move |on_drag_end: On<Pointer<DragEnd>>, mut query: Query<(&mut UiTransform, &mut GlobalZIndex)>| {

                            if let Ok((mut transform, mut global_zindex)) = query.get_mut(on_drag_end.entity()) {
                                transform.translation = Val2::ZERO;
                                global_zindex.0 = 0;
                            }});
                            item_node.with_child((
                                ImageNode {
                                    image: assets.load(item_image_path),
                                    image_mode: NodeImageMode::Auto,
                                    ..Default::default()
                                },
                                Pickable::IGNORE,
                            ));
                        }
                    });
                });
            }
        }
        }).id();
        parent.spawn(Node {
                            column_gap: Val::Px(4. * GAP),
                        ..Default::default()
        }).add_children(&[left, right]);
    });
}
