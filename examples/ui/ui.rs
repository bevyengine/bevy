use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

/// This example illustrates the various features of Bevy UI.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(mouse_scroll)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());

    // root node
    commands
        .spawn_bundle(root_element())
        .with_children(|parent| {
            // left vertical fill (border)
            parent
                .spawn_bundle(left_vertical_fill_border())
                .with_children(|parent| {
                    // left vertical fill (content)
                    parent
                        .spawn_bundle(left_vertical_fill_content())
                        .with_children(|parent| {
                            // text
                            parent.spawn_bundle(left_text(&asset_server));
                        });
                });
            // right vertical fill
            parent
                .spawn_bundle(right_vertical_fill())
                .with_children(|parent| {
                    // Title
                    parent.spawn_bundle(list_title(&asset_server));
                    // List with hidden overflow
                    parent
                        .spawn_bundle(list_with_hidden_overflow())
                        .with_children(|parent| {
                            // Moving panel
                            parent
                                .spawn_bundle(moving_panel())
                                .insert(ScrollingList::default())
                                .with_children(|parent| {
                                    // List items
                                    for i in 0..30 {
                                        parent.spawn_bundle(scroll_list_entry(&asset_server, i));
                                    }
                                });
                        });
                });
            // absolute positioning
            parent
                .spawn_bundle(absolute_positioning_outer_box())
                .with_children(|parent| {
                    parent.spawn_bundle(absolute_positioning_inner_box());
                });
            // render order test: reddest in the back, whitest in the front (flex center)
            parent
                .spawn_bundle(render_test_container())
                .with_children(|parent| {
                    parent
                        .spawn_bundle(color_box(Color::rgb(1.0, 0.0, 0.0)))
                        .with_children(|parent| {
                            parent.spawn_bundle(offset_color_box(
                                20.0,
                                20.0,
                                Color::rgb(1.0, 0.3, 0.3),
                            ));
                            parent.spawn_bundle(offset_color_box(
                                40.0,
                                40.0,
                                Color::rgb(1.0, 0.5, 0.7),
                            ));
                            parent.spawn_bundle(offset_color_box(
                                60.0,
                                60.0,
                                Color::rgb(1.0, 0.7, 0.7),
                            ));
                            // alpha test
                            parent.spawn_bundle(offset_color_box(
                                80.0,
                                80.0,
                                Color::rgba(1.0, 0.9, 0.9, 0.4),
                            ));
                        });
                });
            // bevy logo (flex center)
            parent
                .spawn_bundle(logo_container())
                .with_children(|parent| {
                    // bevy logo (image)
                    parent.spawn_bundle(logo_image(&asset_server));
                });
        });
}

fn absolute_positioning_inner_box() -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            ..Default::default()
        },
        color: Color::rgb(0.8, 0.8, 1.0).into(),
        ..Default::default()
    }
}

fn absolute_positioning_outer_box() -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Px(200.0), Val::Px(200.0)),
            position_type: PositionType::Absolute,
            position: Rect {
                left: Val::Px(210.0),
                bottom: Val::Px(10.0),
                ..Default::default()
            },
            border: Rect::all(Val::Px(20.0)),
            ..Default::default()
        },
        color: Color::rgb(0.4, 0.4, 1.0).into(),
        ..Default::default()
    }
}

fn logo_image(asset_server: &Res<AssetServer>) -> ImageBundle {
    ImageBundle {
        style: Style {
            size: Size::new(Val::Px(500.0), Val::Auto),
            ..Default::default()
        },
        image: asset_server.load("branding/bevy_logo_dark_big.png").into(),
        ..Default::default()
    }
}

fn logo_container() -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexEnd,
            ..Default::default()
        },
        color: Color::NONE.into(),
        ..Default::default()
    }
}

fn color_box(color: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Px(100.0), Val::Px(100.0)),
            ..Default::default()
        },
        color: color.into(),
        ..Default::default()
    }
}

fn offset_color_box(left: f32, bottom: f32, color: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Px(100.0), Val::Px(100.0)),
            position_type: PositionType::Absolute,
            position: Rect {
                left: Val::Px(left),
                bottom: Val::Px(bottom),
                ..Default::default()
            },
            ..Default::default()
        },
        color: color.into(),
        ..Default::default()
    }
}

fn render_test_container() -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        color: Color::NONE.into(),
        ..Default::default()
    }
}

fn right_vertical_fill() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::ColumnReverse,
            justify_content: JustifyContent::Center,
            size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
            ..Default::default()
        },
        color: Color::rgb(0.15, 0.15, 0.15).into(),
        ..Default::default()
    }
}

fn left_text(asset_server: &Res<AssetServer>) -> TextBundle {
    TextBundle {
        style: Style {
            margin: Rect::all(Val::Px(5.0)),
            ..Default::default()
        },
        text: Text::with_section(
            "Text Example",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                color: Color::WHITE,
            },
            Default::default(),
        ),
        ..Default::default()
    }
}

fn left_vertical_fill_content() -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            align_items: AlignItems::FlexEnd,
            ..Default::default()
        },
        color: Color::rgb(0.15, 0.15, 0.15).into(),
        ..Default::default()
    }
}

fn left_vertical_fill_border() -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
            border: Rect::all(Val::Px(2.0)),
            ..Default::default()
        },
        color: Color::rgb(0.65, 0.65, 0.65).into(),
        ..Default::default()
    }
}

fn root_element() -> NodeBundle {
    NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            justify_content: JustifyContent::SpaceBetween,
            ..Default::default()
        },
        color: Color::NONE.into(),
        ..Default::default()
    }
}

fn list_title(asset_server: &Res<AssetServer>) -> TextBundle {
    TextBundle {
        style: Style {
            size: Size::new(Val::Undefined, Val::Px(25.)),
            margin: Rect {
                left: Val::Auto,
                right: Val::Auto,
                ..Default::default()
            },
            ..Default::default()
        },
        text: Text::with_section(
            "Scrolling list",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 25.,
                color: Color::WHITE,
            },
            Default::default(),
        ),
        ..Default::default()
    }
}

fn list_with_hidden_overflow() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::ColumnReverse,
            align_self: AlignSelf::Center,
            size: Size::new(Val::Percent(100.0), Val::Percent(50.0)),
            overflow: Overflow::Hidden,
            ..Default::default()
        },
        color: Color::rgb(0.10, 0.10, 0.10).into(),
        ..Default::default()
    }
}

fn moving_panel() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::ColumnReverse,
            flex_grow: 1.0,
            max_size: Size::new(Val::Undefined, Val::Undefined),
            ..Default::default()
        },
        color: Color::NONE.into(),
        ..Default::default()
    }
}

#[derive(Component, Default)]
struct ScrollingList {
    position: f32,
}

fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Children, &Node)>,
    query_item: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.iter() {
        for (mut scrolling_list, mut style, children, uinode) in query_list.iter_mut() {
            let items_height: f32 = children
                .iter()
                .map(|entity| query_item.get(*entity).unwrap().size.y)
                .sum();
            let panel_height = uinode.size.y;
            let max_scroll = (items_height - panel_height).max(0.);
            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };
            scrolling_list.position += dy;
            scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
            style.position.top = Val::Px(scrolling_list.position);
        }
    }
}

fn scroll_list_entry(asset_server: &AssetServer, i: i32) -> TextBundle {
    TextBundle {
        style: Style {
            flex_shrink: 0.,
            size: Size::new(Val::Undefined, Val::Px(20.)),
            margin: Rect {
                left: Val::Auto,
                right: Val::Auto,
                ..Default::default()
            },
            ..Default::default()
        },
        text: Text::with_section(
            format!("Item {}", i),
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 20.,
                color: Color::WHITE,
            },
            Default::default(),
        ),
        ..Default::default()
    }
}
