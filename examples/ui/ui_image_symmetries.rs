//! Demonstrates how to rotate and flip UI images

use bevy::prelude::*;

#[derive(Component)]
struct OrientationText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 32.,
        color: Color::WHITE,
    };
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn(ImageBundle {
                image: UiImage::new(asset_server.load("branding/icon.png")),
                ..Default::default()
            });

            builder.spawn((
                TextBundle {
                    style: Style {
                        margin: UiRect {
                            top: Val::Px(20.),
                            bottom: Val::Px(20.),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    text: Text::from_section(
                        format!("{:?}", UiContentOrientation::default()),
                        text_style.clone(),
                    ),
                    ..Default::default()
                },
                OrientationText,
            ));

            builder.spawn(TextBundle {
                text: Text::from_section(
                    "Z => rotate clockwise\nX => rotate counterclockwise\nC => flip x\nV => flip y",
                    text_style,
                ),
                ..Default::default()
            });
        });
}

fn update_orientation(
    input: Res<Input<KeyCode>>,
    mut orientation_query: Query<&mut UiContentOrientation, With<UiImage>>,
) {
    let mut orientation = orientation_query.single_mut();
    if input.just_pressed(KeyCode::Z) {
        *orientation = orientation.rotate_left();
    }
    if input.just_pressed(KeyCode::X) {
        *orientation = orientation.rotate_right();
    }
    if input.just_pressed(KeyCode::C) {
        *orientation = orientation.flip_x();
    }
    if input.just_pressed(KeyCode::V) {
        *orientation = orientation.flip_y();
    }
}

fn update_text(
    orientation_query: Query<&UiContentOrientation, (With<UiImage>, Changed<UiContentOrientation>)>,
    mut text_query: Query<&mut Text, With<OrientationText>>,
) {
    if let Ok(orientation) = orientation_query.get_single() {
        let mut text = text_query.single_mut();
        text.sections[0].value = format!("{:?}", orientation);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_orientation, update_text))
        .run();
}
