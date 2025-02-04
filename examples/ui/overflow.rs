//! Simple example demonstrating overflow behavior.

use bevy::{color::palettes::css::*, prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, update_outlines)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let text_style = TextFont::default();

    let image = asset_server.load("branding/icon.png");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_wrap: FlexWrap::Wrap,
                ..Default::default()
            },
            BackgroundColor(ANTIQUE_WHITE.into()),
        ))
        .with_children(|parent| {
            for overflow in [
                Overflow::visible(),
                Overflow::clip_x(),
                Overflow::clip_y(),
                Overflow::clip(),
                Overflow::hidden_x(),
                Overflow::hidden_y(),
                Overflow::hidden(),
                Overflow {
                    x: OverflowAxis::Hidden,
                    y: OverflowAxis::Clip,
                },
            ] {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        margin: UiRect::horizontal(Val::Px(25.)),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        let label = format!("{overflow:#?}");
                        parent
                            .spawn((
                                Node {
                                    padding: UiRect::all(Val::Px(10.)),
                                    margin: UiRect::bottom(Val::Px(25.)),
                                    ..Default::default()
                                },
                                BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                            ))
                            .with_children(|parent| {
                                parent.spawn((Text::new(label), text_style.clone()));
                            });
                        parent
                            .spawn((
                                Node {
                                    width: Val::Px(100.),
                                    height: Val::Px(100.),
                                    padding: UiRect {
                                        left: Val::Px(25.),
                                        top: Val::Px(25.),
                                        ..Default::default()
                                    },
                                    border: UiRect::all(Val::Px(5.)),
                                    overflow,
                                    ..default()
                                },
                                BorderColor(Color::BLACK),
                                BackgroundColor(GRAY.into()),
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    ImageNode::new(image.clone()),
                                    Node {
                                        min_width: Val::Px(100.),
                                        min_height: Val::Px(100.),
                                        ..default()
                                    },
                                    Interaction::default(),
                                ));
                            });
                    });
            }
        });
}

fn update_outlines(mut interactions_query: Query<(&mut ImageNode, Ref<Interaction>)>) {
    for (mut image_node, interaction) in interactions_query.iter_mut() {
        if interaction.is_changed() {
            image_node.color = match *interaction {
                Interaction::Pressed => RED.into(),
                Interaction::Hovered => YELLOW.into(),
                Interaction::None => Color::WHITE,
            };
        }
    }
}
