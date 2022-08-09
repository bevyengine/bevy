//! This example illustrates how to create an imagem button that changes image offset based on its
//! interaction state.

use bevy::{prelude::*, sprite::Rect, winit::WinitSettings};

fn main() {
    App::new()
        // Change image filter to a pixel-art friendly
        .insert_resource(ImageSettings::default_nearest())
        // Match the background color with base image color
        .insert_resource(ClearColor(Color::rgb(0.475, 0.239, 0.306)))
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .add_system(button_system)
        .run();
}

// Image rect in pixels, inside the base image.
// Values are using to built a rect with begin (X, Y) and end (X, Y) format
const HOVERED_BUTTON_OFFSET: [f32; 4] = [23.0, 38.0, 36.0, 52.0];
const NORMAL_BUTTON_OFFSET: [f32; 4] = [7.0, 38.0, 20.0, 52.0];
const CLICKED_BUTTON_OFFSET: [f32; 4] = [39.0, 38.0, 52.0, 52.0];

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut UiImage),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut ui_image) in &mut interaction_query {
        let offset = match *interaction {
            Interaction::Hovered => NORMAL_BUTTON_OFFSET,
            Interaction::None => HOVERED_BUTTON_OFFSET,
            Interaction::Clicked => CLICKED_BUTTON_OFFSET,
        };

        ui_image.offset = Rect {
            min: Vec2::new(offset[0], offset[1]),
            max: Vec2::new(offset[2], offset[3]),
        };
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn_bundle(Camera2dBundle::default());

    // image button
    commands.spawn_bundle(ButtonBundle {
        style: Style {
            size: Size::new(Val::Px(150.0), Val::Px(150.0)),
            // center button
            margin: UiRect::all(Val::Auto),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            ..default()
        },
        // Default image has no offset, but that's OK since it'll be update it on button_system
        image: asset_server
            .load("textures/rpg/ui/generic-rpg-ui-inventario.png")
            .into(),
        ..default()
    });
}
