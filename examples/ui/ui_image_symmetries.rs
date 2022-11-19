//! Demonstrates how to rotate and flip UI images

use bevy::prelude::*;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                size: Size::new(Val::Percent(100.), Val::Percent(100.)),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn(ImageBundle {
                image: UiImage::new(asset_server.load("branding/icon.png")),
                ..Default::default()
            });

            builder.spawn(TextBundle {
                style: Style {
                    margin: UiRect::top(Val::Px(20.)),
                    ..Default::default()
                },
                text: Text::from_section(
                    "1: rotate = false\n2: flip_x = false\n3: flip_y = false",
                    TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 32.,
                        color: Color::WHITE,
                    },
                ),
                ..Default::default()
            });
        });
}

fn update(
    input: Res<Input<KeyCode>>,
    mut image_query: Query<&mut UiImage>,
    mut text_query: Query<&mut Text>,
) {
    let mut image = image_query.single_mut();
    if input.just_pressed(KeyCode::Key1) {
        image.rotate = !image.rotate;
    }
    if input.just_pressed(KeyCode::Key2) {
        image.flip_x = !image.flip_x;
    }
    if input.just_pressed(KeyCode::Key3) {
        image.flip_y = !image.flip_y;
    }

    if image.is_changed() {
        let mut text = text_query.single_mut();
        text.sections[0].value = format!(
            "1: rotate = {}\n2: flip_x = {}\n3: flip_y = {}",
            image.rotate, image.flip_x, image.flip_y
        );
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update)
        .run();
}
