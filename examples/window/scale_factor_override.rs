//! This example illustrates how to override the window scale factor imposed by the
//! operating system.

use bevy::{prelude::*, window::{PrimaryWindow, WindowResolution}};

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 500.,
            height: 300.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(toggle_override)
        .add_system(change_scale_factor)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // camera
    commands.spawn_bundle(Camera2dBundle::default());
    // root node
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            color: Color::NONE.into(),
            ..default()
        })
        .with_children(|parent| {
            // left vertical fill (border)
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    color: Color::rgb(0.65, 0.65, 0.65).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        style: Style {
                            align_self: AlignSelf::FlexEnd,
                            ..default()
                        },
                        text: Text::with_section(
                            "Example text",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                            Default::default(),
                        ),
                        ..default()
                    });
                });
        });
}

/// This system toggles scale factor overrides when enter is pressed
fn toggle_override(
    mut commands: Commands,
    input: Res<Input<KeyCode>>,
    mut primary_window: Res<PrimaryWindow>,
    windows: Query<&WindowResolution, With<Window>>,
) {
    let mut window_commands = commands.window(primary_window.window.unwrap());
    let resolution = windows.get(primary_window.window.unwrap()).unwrap();
    if input.just_pressed(KeyCode::Return) {
        window_commands
            .set_scale_factor_override(resolution.scale_factor_override().xor(Some(1.))); // This is the thing responsible for the toggle
    }
}

/// This system changes the scale factor override when up or down is pressed
fn change_scale_factor(
    mut commands: Commands,
    input: Res<Input<KeyCode>>, 
    mut primary_window: Res<PrimaryWindow>,
    windows: Query<&WindowResolution, With<Window>>
) {
    let mut window_commands = commands.window(primary_window.window.unwrap());
    let resolution = windows.get(primary_window.window.unwrap()).unwrap();
    if input.just_pressed(KeyCode::Up) {
        window_commands
            .set_scale_factor_override(resolution.scale_factor_override().map(|n| n + 1.));
    } else if input.just_pressed(KeyCode::Down) {
        window_commands
            .set_scale_factor_override(resolution.scale_factor_override().map(|n| (n - 1.).max(1.)));
    }
}
