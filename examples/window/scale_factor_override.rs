//! This example illustrates how to override the window scale factor imposed by the
//! operating system.

use bevy::{prelude::*, window::WindowResolution};

#[derive(Component)]
struct CustomText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(500., 300.).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (display_override, toggle_override, change_scale_factor),
        )
        .run();
}

fn setup(mut commands: Commands) {
    // camera
    commands.spawn(Camera2dBundle::default());
    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
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
                        width: Val::Px(300.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: Color::srgb(0.65, 0.65, 0.65).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        CustomText,
                        TextBundle::from_section(
                            "Example text",
                            TextStyle {
                                font_size: 30.0,
                                ..default()
                            },
                        )
                        .with_style(Style {
                            align_self: AlignSelf::FlexEnd,
                            ..default()
                        }),
                    ));
                });
        });
}

/// Set the title of the window to the current override
fn display_override(
    mut windows: Query<&mut Window>,
    mut custom_text: Query<&mut Text, With<CustomText>>,
) {
    let mut window = windows.single_mut();

    let text = format!(
        "Scale factor: {:.1} {}",
        window.scale_factor(),
        if window.resolution.scale_factor_override().is_some() {
            "(overridden)"
        } else {
            "(default)"
        }
    );

    window.title.clone_from(&text);

    let mut custom_text = custom_text.single_mut();
    custom_text.sections[0].value = text;
}

/// This system toggles scale factor overrides when enter is pressed
fn toggle_override(input: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();

    if input.just_pressed(KeyCode::Enter) {
        let scale_factor_override = window.resolution.scale_factor_override();
        window
            .resolution
            .set_scale_factor_override(scale_factor_override.xor(Some(1.0)));
    }
}

/// This system changes the scale factor override when up or down is pressed
fn change_scale_factor(input: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    let scale_factor_override = window.resolution.scale_factor_override();
    if input.just_pressed(KeyCode::ArrowUp) {
        window
            .resolution
            .set_scale_factor_override(scale_factor_override.map(|n| n + 1.0));
    } else if input.just_pressed(KeyCode::ArrowDown) {
        window
            .resolution
            .set_scale_factor_override(scale_factor_override.map(|n| (n - 1.0).max(1.0)));
    }
}
