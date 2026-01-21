//! This example illustrates the advance usage of an image node.
//! Compare `NodeImageMode` behavior

use bevy::{color::palettes::tailwind, prelude::*};

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

#[derive(Debug, Component)]
struct TextMeta {
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
    commands.spawn(Camera2d);
    // Keyboard Hint
    commands
        .spawn((
            TextMeta { height: 40.,  width : 100. },
            Text::new(
                "Compare NodeImageMode(Auto, Stretch) press `Upload`/`Down` to resize height, press `Left`/`Right` to resize width\nheight : 10%",
            ),
            TextColor::WHITE,
            Node {
                position_type: PositionType::Absolute,
                top: px(4),
                left: px(4),
                ..default()
            },
        ))
        .observe(update_text);

    commands
        .spawn((Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceAround,
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(Val::Px(10.)),
            ..default()
        },))
        .with_children(|builder| {
            // `NodeImageMode::Auto` will be sized automatically by taking the size of the source image and applying any layout constraints.
            builder
                .spawn((
                    ImageGroup,
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::Start,
                        width: Val::Percent(100.),
                        height: Val::Percent(40.),
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
            // `NodeImageMode::Stretch` will resized to match the size of the `Node` component
            builder
                .spawn((
                    ImageGroup,
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::Start,
                        width: Val::Percent(100.),
                        height: Val::Percent(40.),
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

// trigger event
fn update(
    keycode: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    query: Query<Entity, With<TextMeta>>,
) {
    let entity = query.single().unwrap();
    if keycode.pressed(KeyCode::ArrowUp) {
        commands.trigger(ImageGroupResize::HeightGrow);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Height,
            change: 1.,
        });
    }
    if keycode.pressed(KeyCode::ArrowDown) {
        commands.trigger(ImageGroupResize::HeightShrink);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Height,
            change: -1.,
        });
    }
    if keycode.pressed(KeyCode::ArrowLeft) {
        commands.trigger(ImageGroupResize::WidthGrow);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Width,
            change: -1.,
        });
    }
    if keycode.pressed(KeyCode::ArrowRight) {
        commands.trigger(ImageGroupResize::WidthShrink);
        commands.trigger(TextUpdate {
            entity,
            direction: Direction::Width,
            change: 1.,
        });
    }
}

fn update_text(
    event: On<TextUpdate>,
    mut textmeta: Single<&mut TextMeta>,
    mut text: Single<&mut Text>,
) {
    let str = "Compare NodeImageMode(Auto, Stretch) press `Upload`/`Down` to resize height, press `Left`/`Right` to resize width\n";
    let mut new_text = Text::new(str);
    match event.direction {
        Direction::Height => {
            textmeta.height = (textmeta.height + event.change).clamp(10.0, 40.0);
            new_text.push_str(&format!(
                "height : {}%, width : {}%",
                textmeta.height, textmeta.width
            ));
        }
        Direction::Width => {
            textmeta.width = (textmeta.width + event.change).clamp(40.0, 100.0);
            new_text.push_str(&format!(
                "height : {}%, width : {}%",
                textmeta.height, textmeta.width
            ));
        }
    }
    text.0 = new_text.0;
}

static MINIUM_RESIZE_VAL: f32 = 1.0;
static IMAGEG_GROUP_BOX_MIN_WIDTH: f32 = 50.0;
static IMAGEG_GROUP_BOX_MAX_WIDTH: f32 = 100.0;
static IMAGEG_GROUP_BOX_MIN_HEIGHT: f32 = 10.0;
static IMAGEG_GROUP_BOX_MAX_HEIGHT: f32 = 40.0;

fn on_trigger_image_group(event: On<ImageGroupResize>, query: Query<&mut Node, With<ImageGroup>>) {
    for mut node in query {
        match event.event() {
            ImageGroupResize::HeightGrow => {
                if let Val::Percent(val) = node.height {
                    let new_val = (val + MINIUM_RESIZE_VAL).min(IMAGEG_GROUP_BOX_MAX_HEIGHT);
                    node.height = Val::Percent(new_val);
                }
            }
            ImageGroupResize::HeightShrink => {
                if let Val::Percent(val) = node.height {
                    let new_val = (val - MINIUM_RESIZE_VAL).max(IMAGEG_GROUP_BOX_MIN_HEIGHT);
                    node.height = Val::Percent(new_val);
                }
            }
            ImageGroupResize::WidthGrow => {
                if let Val::Percent(val) = node.height {
                    let new_val = (val + MINIUM_RESIZE_VAL).min(IMAGEG_GROUP_BOX_MAX_WIDTH);
                    node.height = Val::Percent(new_val);
                }
            }
            ImageGroupResize::WidthShrink => {
                if let Val::Percent(val) = node.height {
                    let new_val = (val - MINIUM_RESIZE_VAL).max(IMAGEG_GROUP_BOX_MIN_WIDTH);
                    node.height = Val::Percent(new_val);
                }
            }
        }
    }
}
