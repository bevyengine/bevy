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
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle::default();

    let image = asset_server.load("branding/icon.png");

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            background_color: ANTIQUE_WHITE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            for overflow in [
                Overflow::visible(),
                Overflow::clip_x(),
                Overflow::clip_y(),
                Overflow::clip(),
            ] {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            margin: UiRect::horizontal(Val::Px(25.)),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        let label = format!("{overflow:#?}");
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    padding: UiRect::all(Val::Px(10.)),
                                    margin: UiRect::bottom(Val::Px(25.)),
                                    ..Default::default()
                                },
                                background_color: Color::srgb(0.25, 0.25, 0.25).into(),
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent.spawn(TextBundle {
                                    text: Text::from_section(label, text_style.clone()),
                                    ..Default::default()
                                });
                            });
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    width: Val::Px(100.),
                                    height: Val::Px(100.),
                                    padding: UiRect {
                                        left: Val::Px(25.),
                                        top: Val::Px(25.),
                                        ..Default::default()
                                    },
                                    overflow,
                                    ..Default::default()
                                },
                                background_color: GRAY.into(),
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent.spawn((
                                    ImageBundle {
                                        image: UiImage::new(image.clone()),
                                        style: Style {
                                            min_width: Val::Px(100.),
                                            min_height: Val::Px(100.),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    Interaction::default(),
                                    Outline {
                                        width: Val::Px(2.),
                                        offset: Val::Px(2.),
                                        color: Color::NONE,
                                    },
                                ));
                            });
                    });
            }
        });
}

fn update_outlines(mut outlines_query: Query<(&mut Outline, Ref<Interaction>)>) {
    for (mut outline, interaction) in outlines_query.iter_mut() {
        if interaction.is_changed() {
            outline.color = match *interaction {
                Interaction::Pressed => RED.into(),
                Interaction::Hovered => WHITE.into(),
                Interaction::None => Color::NONE,
            };
        }
    }
}
