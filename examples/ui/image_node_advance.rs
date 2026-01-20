//! This example illustrates the advance usage of an image node.
//! Compare `NodeImageMode` behaviour

use bevy::{color::palettes::tailwind, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // enable for image outline
        .insert_resource(UiDebugOptions {
            enabled: true,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        // observer for enlarge ImageGroup height
        .add_observer(
            |_: On<ImageGroupHeightEnlarge>, query: Query<&mut Node, With<ImageGroup>>| {
                for mut node in query {
                    if let Val::Percent(val) = node.height {
                        let new_val = (val + 1.).min(40.0);
                        node.height = Val::Percent(new_val);
                    }
                }
            },
        )
        // observer for earrow ImageGroup height
        .add_observer(
            |_: On<ImageGroupHeightNarrow>, query: Query<&mut Node, With<ImageGroup>>| {
                for mut node in query {
                    if let Val::Percent(val) = node.height {
                        let new_val = (val - 1.).max(10.);
                        node.height = Val::Percent(new_val);
                    }
                }
            },
        )
        // observer for enlarge ImageGroup width
        .add_observer(
            |_: On<ImageGroupWidthEnlarge>, query: Query<&mut Node, With<ImageGroup>>| {
                for mut node in query {
                    if let Val::Percent(val) = node.width {
                        let new_val = (val - 1.).max(40.);
                        node.width = Val::Percent(new_val);
                    }
                }
            },
        )
        // observer for earrow ImageGroup width
        .add_observer(
            |_: On<ImageGroupWidthNarrow>, query: Query<&mut Node, With<ImageGroup>>| {
                for mut node in query {
                    if let Val::Percent(val) = node.width {
                        let new_val = (val + 1.).min(100.);
                        node.width = Val::Percent(new_val);
                    }
                }
            },
        )
        .run();
}

#[derive(Debug, Component)]
struct ImageGroup;

#[derive(Debug, Event)]
struct ImageGroupHeightEnlarge;

#[derive(Debug, Event)]
struct ImageGroupHeightNarrow;

#[derive(Debug, Event)]
struct ImageGroupWidthEnlarge;

#[derive(Debug, Event)]
struct ImageGroupWidthNarrow;

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
struct TextUpdateEvent {
    entity: Entity,
    direction: Direction,
    change: f32,
}

// press `h/H` and `↑`/`↓` to resize height
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
        commands.trigger(ImageGroupHeightEnlarge);
        commands.trigger(TextUpdateEvent {
            entity,
            direction: Direction::Height,
            change: 1.,
        });
    }
    if keycode.pressed(KeyCode::ArrowDown) {
        commands.trigger(ImageGroupHeightNarrow);
        commands.trigger(TextUpdateEvent {
            entity,
            direction: Direction::Height,
            change: -1.,
        });
    }
    if keycode.pressed(KeyCode::ArrowLeft) {
        commands.trigger(ImageGroupWidthEnlarge);
        commands.trigger(TextUpdateEvent {
            entity,
            direction: Direction::Width,
            change: -1.,
        });
    }
    if keycode.pressed(KeyCode::ArrowRight) {
        commands.trigger(ImageGroupWidthNarrow);
        commands.trigger(TextUpdateEvent {
            entity,
            direction: Direction::Width,
            change: 1.,
        });
    }
}

fn update_text(
    event: On<TextUpdateEvent>,
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
