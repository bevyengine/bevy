//! Demonstrates how UI elements can be layed-out independently of their ancestors.
//! 
//! Do note that this solution is only but sugar, it is recomended that you make dialogs direct childrens of the root UI Node.

use bevy::color::palettes::css::{ALICE_BLUE, CRIMSON};
use bevy::prelude::*;
use bevy::text::FontSmoothing;
use bevy::winit::WinitSettings;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 24.0,
        font_smoothing: FontSmoothing::AntiAliased,
    };

    commands.spawn(Camera2d::default());
    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceEvenly,
            ..Default::default()
        },
        BackgroundColor(Color::BLACK),
    )).with_children(|parent| {

        let ui_root = parent.parent_entity();

        parent.spawn((
            Text::new("Notice how the dialog box is layed-out independently from it's parent."),
            text_style.clone(),
            TextColor(Color::WHITE),
        ));

        parent
            .spawn((
                Node {
                    align_items: AlignItems::Default,
                    left: Val::Percent(0.),
                    top: Val::Percent(0.),
                    width: Val::Percent(50.),
                    height: Val::Percent(50.),
                    ..Default::default()
                },
                BackgroundColor(Color::Srgba(ALICE_BLUE)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Node {
                        // We use PositionType::Fixed to create a dialog box
                        position_type: PositionType::Fixed(ui_root),
                        align_items: AlignItems::Center,
                        left: Val::Percent(0.),
                        top: Val::Percent(0.),
                        width: Val::Auto,
                        height: Val::Percent(50.),
                        ..Default::default()
                    },
                    BackgroundColor(Color::Srgba(CRIMSON)),
                    Text::new("Dialog content"),
                    text_style.clone(),
                    TextColor(Color::BLACK),
                ));
            });
    });
}
