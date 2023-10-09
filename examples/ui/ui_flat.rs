//! This example illustrates the various features of Bevy UI.

use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, mouse_scroll)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Root Node
    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .id();

    // Bevy Logo (flex center)
    let bevy_logo = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            ..default()
        })
        .id();

    // Bevy Logo (image)
    // A `NodeBundle` is used to display the logo the image as an `ImageBundle` can't automatically
    // size itself with a child node present.
    let bevy_logo_image = commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(500.0),
                    height: Val::Px(125.0),
                    margin: UiRect::top(Val::VMin(5.)),
                    ..default()
                },
                // a `NodeBundle` is transparent by default, so to see the image we have to its color to `WHITE`
                background_color: Color::WHITE.into(),
                ..default()
            },
            UiImage::new(asset_server.load("branding/bevy_logo_dark_big.png")),
        ))
        .id();

    // Bevy Logo (alt text)
    // This UI node takes up no space in the layout and the `Text` component is used by the accessibility module
    // and is not rendered.
    let bevy_logo_image_alt = commands
        .spawn((
            NodeBundle {
                style: Style {
                    display: Display::None,
                    ..Default::default()
                },
                ..Default::default()
            },
            Text::from_section("Bevy logo", TextStyle::default()),
        ))
        .id();

    // left vertical fill (border)
    let left = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(200.),
                border: UiRect::all(Val::Px(2.)),
                ..default()
            },
            background_color: Color::rgb(0.65, 0.65, 0.65).into(),
            ..default()
        })
        .id();

    // left vertical fill (content)
    let left_content = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                ..default()
            },
            background_color: Color::rgb(0.15, 0.15, 0.15).into(),
            ..default()
        })
        .id();

    // left vertical (content text)
    let left_content_text = commands
        .spawn((
            TextBundle::from_section(
                "Text Example",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            )
            .with_style(Style {
                margin: UiRect::all(Val::Px(5.)),
                ..default()
            }),
            // Because this is a distinct label widget and
            // not button/list item text, this is necessary
            // for accessibility to treat the text accordingly.
            Label,
        ))
        .id();

    // right vertical fill
    let right = commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Px(200.),
                ..default()
            },
            background_color: Color::rgb(0.15, 0.15, 0.15).into(),
            ..default()
        })
        .id();

    // Right Title
    let right_title = commands
        .spawn((
            TextBundle::from_section(
                "Scrolling list",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 25.,
                    color: Color::WHITE,
                },
            ),
            Label,
        ))
        .id();

    // Right List
    let right_list = commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::Stretch,
                height: Val::Percent(50.),
                overflow: Overflow::clip_y(),
                ..default()
            },
            background_color: Color::rgb(0.10, 0.10, 0.10).into(),
            ..default()
        })
        .id();

    // Right List Moving Panel
    let right_list_panel = commands
        .spawn((
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            ScrollingList::default(),
            AccessibilityNode(NodeBuilder::new(Role::List)),
        ))
        .id();

    // Right List Panel Items
    let right_list_panel_items = (0..30)
        .map(|i| {
            (
                TextBundle::from_section(
                    format!("Item {i}"),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 20.,
                        color: Color::WHITE,
                    },
                ),
                Label,
                AccessibilityNode(NodeBuilder::new(Role::ListItem)),
            )
        })
        .map(|bundle| commands.spawn(bundle).id())
        .collect::<Vec<_>>();

    // Blue Box
    let blue = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(200.0),
                height: Val::Px(200.0),
                position_type: PositionType::Absolute,
                left: Val::Px(210.),
                bottom: Val::Px(10.),
                border: UiRect::all(Val::Px(20.)),
                ..default()
            },
            border_color: Color::GREEN.into(),
            background_color: Color::rgb(0.4, 0.4, 1.).into(),
            ..default()
        })
        .id();

    // Light Blue Box
    let light_blue = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::rgb(0.8, 0.8, 1.).into(),
            ..default()
        })
        .id();

    // Render Order Test
    let render_test = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .id();

    // Render Order Background
    let render_test_background = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(100.0),
                height: Val::Px(100.0),
                ..default()
            },
            background_color: Color::rgb(1.0, 0.0, 0.).into(),
            ..default()
        })
        .id();

    // Render Order Red 1
    let render_test_red_1 = commands
        .spawn(NodeBundle {
            style: Style {
                // Take the size of the parent node.
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(20.),
                bottom: Val::Px(20.),
                ..default()
            },
            background_color: Color::rgb(1.0, 0.3, 0.3).into(),
            ..default()
        })
        .id();

    // Render Order Red 2
    let render_test_red_2 = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(40.),
                bottom: Val::Px(40.),
                ..default()
            },
            background_color: Color::rgb(1.0, 0.5, 0.5).into(),
            ..default()
        })
        .id();

    // Render Order Red 3
    let render_test_red_3 = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(60.),
                bottom: Val::Px(60.),
                ..default()
            },
            background_color: Color::rgb(1.0, 0.7, 0.7).into(),
            ..default()
        })
        .id();

    // Render Order Red Alpha
    let render_test_red_alpha = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(80.),
                bottom: Val::Px(80.),
                ..default()
            },
            background_color: Color::rgba(1.0, 0.9, 0.9, 0.4).into(),
            ..default()
        })
        .id();

    // Now that all entities have been created, the relationships can be explicitly laid out.

    commands
        .entity(render_test_background)
        .add_child(render_test_red_1)
        .add_child(render_test_red_2)
        .add_child(render_test_red_3)
        .add_child(render_test_red_alpha);

    commands
        .entity(render_test)
        .add_child(render_test_background);

    commands.entity(blue).add_child(light_blue);

    for item in right_list_panel_items {
        commands.entity(right_list_panel).add_child(item);
    }

    commands.entity(right_list).add_child(right_list_panel);

    commands
        .entity(right)
        .add_child(right_title)
        .add_child(right_list);

    commands.entity(left_content).add_child(left_content_text);

    commands.entity(left).add_child(left_content);

    commands
        .entity(bevy_logo_image)
        .add_child(bevy_logo_image_alt);

    commands.entity(bevy_logo).add_child(bevy_logo_image);

    commands
        .entity(root)
        .add_child(left)
        .add_child(right)
        .add_child(blue)
        .add_child(render_test)
        .add_child(bevy_logo);
}

#[derive(Component, Default)]
struct ScrollingList {
    position: f32,
}

fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.iter() {
        for (mut scrolling_list, mut style, parent, list_node) in &mut query_list {
            let items_height = list_node.size().y;
            let container_height = query_node.get(parent.get()).unwrap().size().y;

            let max_scroll = (items_height - container_height).max(0.);

            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };

            scrolling_list.position += dy;
            scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
            style.top = Val::Px(scrolling_list.position);
        }
    }
}
