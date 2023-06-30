use bevy::prelude::*;
use bevy_internal::sprite::Anchor;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, move_anchor_point)
        .add_systems(Update, update)
        .add_systems(Update, handle_button)
        .run();
}

#[derive(Component)]
struct DynamicText;

#[derive(Resource)]
struct TextStyleRes {
    style: TextStyle,
}

#[derive(Component)]
struct AnchorMarker;

#[derive(Component)]
struct AnchorDescriptor;

fn update(
    mut gizmos: Gizmos,
    anchor_pos_query: Query<&Transform, With<DynamicText>>,
    mut anchor_descriptor_query: Query<
        &mut Transform,
        (With<AnchorDescriptor>, Without<DynamicText>), // (Without<DynamicText>, Without<AnchorMarker>, With<Text>),
    >,
) {
    if let Ok(&transform) = anchor_pos_query.get_single() {
        gizmos.ray_2d(
            Vec2::new(transform.translation.x, transform.translation.y),
            Vec2::new(100.0, 245.0),
            Color::RED,
        );
        for mut desc_transform in &mut anchor_descriptor_query {
            *desc_transform = Transform::from_xyz(
                transform.translation.x + 100.0,
                transform.translation.y + 290.0,
                0.0,
            );
        }
    }
}

fn move_anchor_point(
    mut anchor_query: Query<&mut Transform, With<DynamicText>>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut move_dir = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::W) {
        move_dir.y += 150.0 * time.delta_seconds();
    }
    if keyboard_input.pressed(KeyCode::S) {
        move_dir.y -= 150.0 * time.delta_seconds();
    }
    if keyboard_input.pressed(KeyCode::A) {
        move_dir.x -= 150.0 * time.delta_seconds();
    }
    if keyboard_input.pressed(KeyCode::D) {
        move_dir.x += 150.0 * time.delta_seconds();
    }
    for mut transform in &mut anchor_query {
        transform.translation += move_dir;
    }
}

fn handle_button(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    mut text_query: Query<(&mut Text, &mut Anchor), With<DynamicText>>,
    text_style: Res<TextStyleRes>,
) {
    for interaction in interaction_query.iter() {
        match *interaction {
            Interaction::Clicked => {
                for (mut text, mut anchor) in text_query.iter_mut() {
                    match *anchor {
                        Anchor::Center | Anchor::BottomRight => {
                            *anchor = Anchor::BottomLeft;
                            text.sections[0] =
                                TextSection::new("Anchor::BottomLeft", text_style.style.clone());
                        }

                        Anchor::BottomLeft => {
                            *anchor = Anchor::TopLeft;
                            text.sections[0] =
                                TextSection::new("Anchor::TopLeft", text_style.style.clone());
                        }

                        Anchor::TopLeft => {
                            *anchor = Anchor::TopRight;
                            text.sections[0] =
                                TextSection::new("Anchor::TopRight", text_style.style.clone());
                        }

                        Anchor::TopRight => {
                            *anchor = Anchor::BottomRight;
                            text.sections[0] =
                                TextSection::new("Anchor::BottomRight", text_style.style.clone());
                        }
                        _ => panic!("Unexpected anchor found"),
                    }
                }
            }
            Interaction::None | Interaction::Hovered => {}
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut config: ResMut<GizmoConfig>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 50.0,
        color: Color::WHITE,
    };

    commands.insert_resource(TextStyleRes {
        style: text_style.clone(),
    });
    config.line_width = 5.0;
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "Anchor point",
                TextStyle {
                    font_size: 40.0,
                    color: Color::RED,
                    ..default()
                },
            ),
            ..default()
        },
        AnchorDescriptor,
    ));

    commands.spawn((AnchorMarker, Transform::default()));
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Hello, Center!", text_style)
                .with_alignment(TextAlignment::Center),
            text_anchor: Anchor::Center,
            ..default()
        },
        DynamicText,
    ));
    commands.spawn(Text2dBundle {
        text: Text::from_section(
            "Press WASD to move the anchor point.",
            TextStyle {
                font_size: 25.0,
                ..default()
            },
        ),
        transform: Transform::from_xyz(250.0, -250.0, 0.0),
        ..default()
    });

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        width: Val::Px(150.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: Color::GOLD.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Press me!",
                        TextStyle {
                            font: font.clone(),
                            font_size: 25.0,
                            color: Color::PURPLE,
                        },
                    ));
                });
        });
}
