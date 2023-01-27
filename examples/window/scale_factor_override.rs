//! This example illustrates how to override the window scale factor imposed by the
//! operating system.

use bevy::{prelude::*, window::WindowResolution};

#[derive(Resource, Default)]
struct Override(Option<f64>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(500., 300.),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<Override>()
        .add_startup_system(setup)
        .add_system(update_window)
        .add_system(toggle_override)
        .add_system(change_scale_factor)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // camera
    commands.spawn(Camera2dBundle::default());
    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // left vertical fill (border)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: Color::rgb(0.65, 0.65, 0.65).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(
                        TextBundle::from_section(
                            "[Enter] Toggle override\n[↑↓] Change scale",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        )
                        .with_style(Style {
                            align_self: AlignSelf::FlexEnd,
                            ..default()
                        }),
                    );
                });
        });
}

fn update_window(mut windows: Query<&mut Window>, over: Res<Override>) {
    if !over.is_changed() {
        return;
    }

    let mut window = windows.single_mut();

    window.resolution.set_scale_factor_override(over.0);

    window.title = format!(
        "Scale override: {:?}",
        window.resolution.scale_factor_override()
    );
}

/// This system toggles scale factor overrides when enter is pressed
fn toggle_override(input: Res<Input<KeyCode>>, mut over: ResMut<Override>) {
    if input.just_pressed(KeyCode::Return) {
        over.0 = over.0.xor(Some(1.0));
    }
}

/// This system changes the scale factor override when up or down is pressed
fn change_scale_factor(input: Res<Input<KeyCode>>, mut over: ResMut<Override>) {
    if input.just_pressed(KeyCode::Up) {
        over.0 = over.0.map(|n| n + 1.0);
    } else if input.just_pressed(KeyCode::Down) {
        over.0 = over.0.map(|n| (n - 1.0).max(1.0));
    }
}
