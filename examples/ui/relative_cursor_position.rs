//! Showcases the [`RelativeCursorPosition`] component, used to check the position of the cursor relative to a UI node.

use bevy::{prelude::*, ui::RelativeCursorPosition, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, relative_cursor_position_system)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(250.),
                        height: Val::Px(250.),
                        margin: UiRect::bottom(Val::Px(15.)),
                        ..default()
                    },
                    background_color: Color::rgb(235., 35., 12.).into(),
                    ..default()
                })
                .insert(RelativeCursorPosition::default());

            parent.spawn(TextBundle {
                text: Text::from_section(
                    "(0.0, 0.0)",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                ),
                ..default()
            });
        });
}

/// This systems polls the relative cursor position and displays its value in a text component.
fn relative_cursor_position_system(
    relative_cursor_position_query: Query<&RelativeCursorPosition>,
    mut output_query: Query<&mut Text>,
) {
    let relative_cursor_position = relative_cursor_position_query.single();

    let mut output = output_query.single_mut();

    output.sections[0].value =
        if let Some(relative_cursor_position) = relative_cursor_position.normalized {
            format!(
                "({:.1}, {:.1})",
                relative_cursor_position.x, relative_cursor_position.y
            )
        } else {
            "unknown".to_string()
        };

    output.sections[0].style.color = if relative_cursor_position.mouse_over() {
        Color::rgb(0.1, 0.9, 0.1)
    } else {
        Color::rgb(0.9, 0.1, 0.1)
    };
}
