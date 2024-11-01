//! Demonstrates how UI elements can be layed-out independently of their ancestors.
//! 
//! Do note that this solution is only but sugar, it is recomended that you make dialogs direct childrens of the root UI Node.

use bevy::prelude::*;
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
    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 24.0,
        color: Color::WHITE,
    };

    commands.spawn(Camera2dBundle::default());
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceEvenly,
            ..default()
        },
        background_color: BackgroundColor(Color::BLACK),
        border_color: BorderColor(Color::BLUE),
        ..default()
    }).with_children(|parent| {

        let ui_root = parent.parent_entity();

        parent.spawn((TextBundle {
            text: Text::from_section(
                "Notice how the dialog box is layed-out independently from it's parent.",                
                text_style.clone(),
            ).with_justify(JustifyText::Center),
            style: Style {
                margin: UiRect::bottom(Val::Px(10.)),
                ..Default::default()
            },   
            ..Default::default()
        },
        ));

        parent
            .spawn(NodeBundle {
                style: Style {
                    align_items: AlignItems::Default,
                    left: Val::Percent(0.),
                    top: Val::Percent(0.),
                    width: Val::Percent(50.),
                    height: Val::Percent(50.),
                    ..Default::default()
                },
                background_color: BackgroundColor(Color::ALICE_BLUE),
                ..Default::default()
            })
            .with_children(|parent| {
                parent.spawn(TextBundle {
                    style: Style {
                        // We use PositionType::Fixed to create a dialog box
                        position_type: PositionType::Fixed(ui_root),
                        align_items: AlignItems::Center,
                        left: Val::Percent(5.),
                        top: Val::Percent(5.),
                        width: Val::Auto,
                        height: Val::Percent(10.),
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::CRIMSON),
                    text: Text::from_section(
                    "Dialog content",                
                        text_style.clone(),
                    ).with_justify(JustifyText::Center),
                    ..Default::default()
                });
            });
    });
}
