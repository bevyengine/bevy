//! This example demonstrates the behavior of `NodeImageMode::Auto` and `NodeImageMode::Stretch` by allowing keyboard input to resize an `ImageGroup` container.
//! It visually shows how images are sized automatically versus stretched to fit their container.

use bevy::{color::palettes::tailwind, prelude::*};

const MIN_RESIZE_VAL: f32 = 1.0;
const IMAGE_GROUP_BOX_MIN_WIDTH: f32 = 50.0;
const IMAGE_GROUP_BOX_MAX_WIDTH: f32 = 100.0;
const IMAGE_GROUP_BOX_MIN_HEIGHT: f32 = 10.0;
const IMAGE_GROUP_BOX_MAX_HEIGHT: f32 = 50.0;
const IMAGE_GROUP_BOX_INIT_WIDTH: f32 =
    (IMAGE_GROUP_BOX_MIN_WIDTH + IMAGE_GROUP_BOX_MAX_WIDTH) / 2.;
const IMAGE_GROUP_BOX_INIT_HEIGHT: f32 =
    (IMAGE_GROUP_BOX_MIN_HEIGHT + IMAGE_GROUP_BOX_MAX_HEIGHT) / 2.;
const TEXT_PREFIX: &str = "Compare NodeImageMode(Auto, Stretch) press `Up`/`Down` to resize height, press `Left`/`Right` to resize width\n";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Enable for image outline
        .insert_resource(UiDebugOptions {
            enabled: true,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .add_observer(on_trigger_image_group)
        .run();
}

#[derive(Debug, Component)]
struct ImageGroup;

#[derive(Debug, Event)]
enum ImageGroupResize {
    HeightGrow,
    HeightShrink,
    WidthGrow,
    WidthShrink,
}

// Text data for easy modification
#[derive(Debug, Component)]
struct TextData {
    height: f32,
    width: f32,
}

#[derive(Debug)]
enum Direction {
    Height,
    Width,
}

#[derive(Debug, EntityEvent)]
struct TextUpdate {
    entity: Entity,
    direction: Direction,
    change: f32,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image_handle = asset_server.load("branding/icon.png");
    let full_text = format!(
        "{}height : {}%, width : {}%",
        TEXT_PREFIX, IMAGE_GROUP_BOX_INIT_HEIGHT, IMAGE_GROUP_BOX_INIT_WIDTH,
    );

    commands.spawn(Camera2d);

    let container = commands
        .spawn((
            Node {
                display: Display::Grid,
                width: percent(100),
                height: percent(100),
                grid_template_rows: vec![GridTrack::min_content(), GridTrack::flex(1.0)],
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .id();

    // Keyboard Text
    commands
        .spawn((
            TextData {
                height: IMAGE_GROUP_BOX_INIT_HEIGHT,
                width: IMAGE_GROUP_BOX_INIT_WIDTH,
            },
            Text::new(full_text),
            TextColor::BLACK,
            Node {
                grid_row: GridPlacement::span(1),
                padding: UiRect::all(px(6)),
                ..default()
            },
            UiDebugOptions {
                enabled: false,
                ..default()
            },
            UiDebugOptions {
                enabled: false,
                ..default()
            },
            ChildOf(container),
        ))
        .observe(update_text);

    commands
        .spawn((
            Node {
                display: Display::Flex,
                grid_row: GridPlacement::span(1),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceAround,
                padding: UiRect::all(Val::Px(10.)),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            ChildOf(container),
        ))
        .with_children(|builder| {
            // `NodeImageMode::Auto` will resize the image automatically by taking the size of the source image and applying any layout constraints.
            builder
                .spawn((
                    ImageGroup,
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::Start,
                        width: Val::Percent(IMAGE_GROUP_BOX_INIT_WIDTH),
                        height: Val::Percent(IMAGE_GROUP_BOX_INIT_HEIGHT),
                        ..default()
                    },
                    BackgroundColor(Color::from(tailwind::BLUE_100)),
                ))
                .with_children(|parent| {
                    for _ in 0..4 {
                        // child node will apply Flex layout
                        parent.spawn((
                            Node::default(),
                            ImageNode {
                                image: image_handle.clone(),
                                image_mode: NodeImageMode::Auto,
                                ..default()
                            },
                        ));
                    }
                });
            // `NodeImageMode::Stretch` will resize the image to match the size of the `Node` component
            builder
                .spawn((
                    ImageGroup,
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::Start,
                        width: Val::Percent(IMAGE_GROUP_BOX_INIT_WIDTH),
                        height: Val::Percent(IMAGE_GROUP_BOX_INIT_HEIGHT),
                        ..default()
                    },
                    BackgroundColor(Color::from(tailwind::BLUE_100)),
                ))
                .with_children(|parent| {
                    for width in [10., 20., 30., 40.] {
                        parent.spawn((
                            Node {
                                height: Val::Percent(100.),
                                width: Val::Percent(width),
                                ..default()
                            },
                            ImageNode {
                                image: image_handle.clone(),
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                        ));
                    }
                });
        });
}

// Trigger event
fn update(
    keycode: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    query: Query<Entity, With<TextData>>,
) {
    let entity = query.single().unwrap();
    if keycode.pressed(KeyCode::ArrowUp) {
        commands.trigger(ImageGroupResize::HeightGrow);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Height,
            change: MIN_RESIZE_VAL,
        });
    }
    if keycode.pressed(KeyCode::ArrowDown) {
        commands.trigger(ImageGroupResize::HeightShrink);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Height,
            change: -MIN_RESIZE_VAL,
        });
    }
    if keycode.pressed(KeyCode::ArrowLeft) {
        commands.trigger(ImageGroupResize::WidthShrink);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Width,
            change: -MIN_RESIZE_VAL,
        });
    }
    if keycode.pressed(KeyCode::ArrowRight) {
        commands.trigger(ImageGroupResize::WidthGrow);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Width,
            change: MIN_RESIZE_VAL,
        });
    }
}

fn update_text(
    event: On<TextUpdate>,
    mut textmeta: Single<&mut TextData>,
    mut text: Single<&mut Text>,
) {
    let mut new_text = Text::new(TEXT_PREFIX);
    match event.direction {
        Direction::Height => {
            textmeta.height = (textmeta.height + event.change)
                .clamp(IMAGE_GROUP_BOX_MIN_HEIGHT, IMAGE_GROUP_BOX_MAX_HEIGHT);
            new_text.push_str(&format!(
                "height : {}%, width : {}%",
                textmeta.height, textmeta.width
            ));
        }
        Direction::Width => {
            textmeta.width = (textmeta.width + event.change)
                .clamp(IMAGE_GROUP_BOX_MIN_WIDTH, IMAGE_GROUP_BOX_MAX_WIDTH);
            new_text.push_str(&format!(
                "height : {}%, width : {}%",
                textmeta.height, textmeta.width
            ));
        }
    }
    text.0 = new_text.0;
}

fn on_trigger_image_group(event: On<ImageGroupResize>, query: Query<&mut Node, With<ImageGroup>>) {
    for mut node in query {
        match event.event() {
            ImageGroupResize::HeightGrow => {
                if let Val::Percent(val) = node.height {
                    let new_val = (val + MIN_RESIZE_VAL).min(IMAGE_GROUP_BOX_MAX_HEIGHT);
                    node.height = Val::Percent(new_val);
                }
            }
            ImageGroupResize::HeightShrink => {
                if let Val::Percent(val) = node.height {
                    let new_val = (val - MIN_RESIZE_VAL).max(IMAGE_GROUP_BOX_MIN_HEIGHT);
                    node.height = Val::Percent(new_val);
                }
            }
            ImageGroupResize::WidthGrow => {
                if let Val::Percent(val) = node.width {
                    let new_val = (val + MIN_RESIZE_VAL).min(IMAGE_GROUP_BOX_MAX_WIDTH);
                    node.width = Val::Percent(new_val);
                }
            }
            ImageGroupResize::WidthShrink => {
                if let Val::Percent(val) = node.width {
                    let new_val = (val - MIN_RESIZE_VAL).max(IMAGE_GROUP_BOX_MIN_WIDTH);
                    node.width = Val::Percent(new_val);
                }
            }
        }
    }
}
