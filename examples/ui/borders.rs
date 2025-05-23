//! Example demonstrating bordered UI nodes

use bevy::ecs::relationship::{AncestorIter, RelationshipSourceCollection};
use bevy::{color::palettes::css::*, ecs::spawn::SpawnIter, prelude::*};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::HoverMap,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_scroll_position)
        .run();
}

const LINE_HEIGHT: f32 = 21.;

/// Updates the scroll position of scrollable nodes in response to mouse input
pub fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scrolled_node_query: Query<(&mut ScrollPosition, &Node)>,
    child_of_query: Query<&ChildOf>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        let (mut dx, mut dy) = match mouse_wheel_event.unit {
            MouseScrollUnit::Line => (
                mouse_wheel_event.x * LINE_HEIGHT,
                mouse_wheel_event.y * LINE_HEIGHT,
            ),
            MouseScrollUnit::Pixel => (mouse_wheel_event.x, mouse_wheel_event.y),
        };
        // accelerate scroll
        // dx *= 5.0;
        // dy *= 5.0;

        if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight)
        {
            std::mem::swap(&mut dx, &mut dy);
        }

        for (_pointer, pointer_map) in hover_map.iter() {
            for (entity, _hit) in pointer_map.iter() {
                let ancestors_iter = (*entity)
                    .iter()
                    .chain(AncestorIter::new(&child_of_query, *entity));

                for ancestor_entity in ancestors_iter {
                    // This may mut scroll position multiple times.
                    // E.g., multiple pointers hovering over the same entity.
                    // TODO collect _distinct_ scroll targets, then apply scroll to them.
                    if let Ok((mut scroll_position, node)) =
                        scrolled_node_query.get_mut(ancestor_entity)
                    {
                        // May be desirable to not capture scroll if there's no more room for scroll.
                        // Although that is to be governed by CSS's overscroll-behavior, TODO.
                        // In this version, targets without scroll axis won't capture that scroll input.
                        // I.e., scroll_x() targets only capture x scroll, y scroll propagates further.
                        if dy != 0.0 && node.overflow.y.is_scroll()
                            || dx != 0.0 && node.overflow.x.is_scroll()
                        {
                            scroll_position.offset_y -= dy;
                            scroll_position.offset_x -= dx;
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d);

    // labels for the different border edges
    let border_labels = [
        "None",
        "All",
        "Left",
        "Right",
        "Top",
        "Bottom",
        "Horizontal",
        "Vertical",
        "Top Left",
        "Bottom Left",
        "Top Right",
        "Bottom Right",
        "Top Bottom Right",
        "Top Bottom Left",
        "Top Left Right",
        "Bottom Left Right",
    ];

    // all the different combinations of border edges
    // these correspond to the labels above
    let borders = [
        UiRect::default(),
        UiRect::all(Val::Px(10.)),
        UiRect::left(Val::Px(10.)),
        UiRect::right(Val::Px(10.)),
        UiRect::top(Val::Px(10.)),
        UiRect::bottom(Val::Px(10.)),
        UiRect::horizontal(Val::Px(10.)),
        UiRect::vertical(Val::Px(10.)),
        UiRect {
            left: Val::Px(20.),
            top: Val::Px(10.),
            ..default()
        },
        UiRect {
            left: Val::Px(10.),
            bottom: Val::Px(20.),
            ..default()
        },
        UiRect {
            right: Val::Px(20.),
            top: Val::Px(10.),
            ..default()
        },
        UiRect {
            right: Val::Px(10.),
            bottom: Val::Px(10.),
            ..default()
        },
        UiRect {
            right: Val::Px(10.),
            top: Val::Px(20.),
            bottom: Val::Px(10.),
            ..default()
        },
        UiRect {
            left: Val::Px(10.),
            top: Val::Px(10.),
            bottom: Val::Px(10.),
            ..default()
        },
        UiRect {
            left: Val::Px(20.),
            right: Val::Px(10.),
            top: Val::Px(10.),
            ..default()
        },
        UiRect {
            left: Val::Px(10.),
            right: Val::Px(10.),
            bottom: Val::Px(20.),
            ..default()
        },
    ];

    let borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            |(label, border)| {
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                border,
                                width: Val::Px(50.),
                                height: Val::Px(50.),
                                display: Display::Flex,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                            children![(
                                Node {
                                    width: Val::Px(10.),
                                    height: Val::Px(10.),
                                    ..default()
                                },
                                BackgroundColor(YELLOW.into()),
                            )]
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let non_zero = |x, y| x != Val::Px(0.) && y != Val::Px(0.);
    let border_size = move |x, y| {
        if non_zero(x, y) {
            f32::MAX
        } else {
            0.
        }
    };

    let borders_examples_rounded = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            move |(label, border)| {
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                border,
                                width: Val::Px(50.),
                                height: Val::Px(50.),
                                display: Display::Flex,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            BorderRadius::px(
                                border_size(border.left, border.top),
                                border_size(border.right, border.top),
                                border_size(border.right, border.bottom,),
                                border_size(border.left, border.bottom),
                            ),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                            children![(
                                Node {
                                    width: Val::Px(10.),
                                    height: Val::Px(10.),
                                    ..default()
                                },
                                BorderRadius::MAX,
                                BackgroundColor(YELLOW.into()),
                            )],
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let empty_borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            |(label, border)| {
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                border,
                                padding: UiRect::all(Val::Px(20.)),
                                display: Display::Flex,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let text_borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            |(label, border)| {
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            Text::new("text\nlines"),
                            TextBackgroundColor::from(PURPLE),
                            TextShadow::default(),
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                border,
                                padding: UiRect::all(Val::Px(20.)),
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let scrollable_text_borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            |(label, border)| {
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            Text::new(
                                "I am a pretty long text line 1\nI am a pretty long text line 2"
                            ),
                            TextBackgroundColor::from(PURPLE),
                            TextShadow::default(),
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                box_sizing: BoxSizing::ContentBox,
                                width: Val::Px(120.),
                                height: Val::Px(120.),
                                border,
                                overflow: Overflow::scroll_y(),
                                padding: UiRect::all(Val::Px(20.)),
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let image = asset_server.load("branding/icon.png");
    let image_borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            move |(label, border)| {
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            ImageNode::new(image.clone()),
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                border,
                                padding: UiRect::all(Val::Px(20.)),
                                width: Val::Px(80.),
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let image = asset_server.load("branding/icon.png");
    let image_content_box_borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            move |(label, border)| {
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            ImageNode::new(image.clone()),
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                box_sizing: BoxSizing::ContentBox,
                                width: Val::Px(80.),
                                border,
                                padding: UiRect::all(Val::Px(20.)),
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let image = asset_server.load("branding/icon.png");
    let flipped_image_content_box_borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(
            border_labels.into_iter().zip(borders).enumerate().map(
                move |(idx, (label, border))| {
                    let flip_pos = idx % 4;
                    (
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        children![
                            (
                                ImageNode {
                                    image: image.clone(),
                                    // flipped: none, y, yx, x; repeat
                                    flip_x: flip_pos == 3 || flip_pos == 2,
                                    flip_y: flip_pos == 1 || flip_pos == 2,
                                    ..default()
                                },
                                Node {
                                    margin: UiRect::all(Val::Px(20.)),
                                    box_sizing: BoxSizing::ContentBox,
                                    width: Val::Px(80.),
                                    border,
                                    padding: UiRect::all(Val::Px(20.)),
                                    ..default()
                                },
                                BackgroundColor(MAROON.into()),
                                BorderColor(RED.into()),
                                Outline {
                                    width: Val::Px(6.),
                                    offset: Val::Px(0.),
                                    color: Color::WHITE,
                                },
                            ),
                            (Text::new(label), TextFont::from_font_size(9.0))
                        ],
                    )
                },
            ),
        )),
    );

    let image = asset_server.load("branding/banner.png");
    let image_rounded_borders_examples = (
        Node {
            margin: UiRect::all(Val::Px(25.0)),
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            move |(label, border)| {
                (
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            ImageNode::new(image.clone()),
                            Node {
                                margin: UiRect::all(Val::Px(20.)),
                                width: Val::Px(120.),
                                border,
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor(RED.into()),
                            BorderRadius::px(
                                border_size(border.left, border.top),
                                border_size(border.right, border.top),
                                border_size(border.right, border.bottom),
                                border_size(border.left, border.bottom),
                            ),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(0.),
                                color: Color::WHITE,
                            },
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    commands.spawn((
        Node {
            margin: UiRect::all(Val::Auto),
            width: Val::Vw(95.0),
            height: Val::Vh(95.0),
            overflow: Overflow::scroll_y(),
            padding: UiRect::all(Val::Px(20.)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        children![
            label("Borders"),
            borders_examples,
            label("Borders Rounded"),
            borders_examples_rounded,
            label("Empty Borders"),
            empty_borders_examples,
            label("Text Borders"),
            text_borders_examples,
            label("Scrollable Text Borders"),
            scrollable_text_borders_examples,
            label("Image Borders"),
            image_borders_examples,
            label("Image Content Box Borders"),
            image_content_box_borders_examples,
            label("Flipped Content Box Images Borders"),
            flipped_image_content_box_borders_examples,
            label("Image Rounded Borders"),
            image_rounded_borders_examples,
        ],
    ));
}

// A label widget that accepts a &str and returns
// a Bundle that can be spawned
fn label(text: &str) -> impl Bundle {
    (
        Node {
            display: Display::Block,
            margin: UiRect::all(Val::Px(25.0)),
            ..default()
        },
        Outline {
            width: Val::Px(6.),
            offset: Val::Px(0.),
            color: Color::WHITE,
        },
        children![(Text::new(text), TextFont::from_font_size(20.0))],
    )
}
