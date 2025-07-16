//! This example illustrates how to override the window scale factor imposed by the
//! operating system.

use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResolution},
};

#[derive(Component)]
struct CustomText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_observer(configure_window)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (display_override, toggle_override, change_scale_factor),
        )
        .run();
}

fn configure_window(trigger: On<Add, PrimaryWindow>, mut window: Query<&mut Window>) {
    let mut window = window.get_mut(trigger.target()).unwrap();
    window.resolution = WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0);
}

fn setup(mut commands: Commands) {
    // camera
    commands.spawn(Camera2d);
    // root node
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|parent| {
            // left vertical fill (border)
            parent
                .spawn((
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.65, 0.65, 0.65)),
                ))
                .with_child((
                    CustomText,
                    Text::new("Example text"),
                    TextFont {
                        font_size: 25.0,
                        ..default()
                    },
                    Node {
                        align_self: AlignSelf::FlexEnd,
                        ..default()
                    },
                ));
        });
}

/// Set the title of the window to the current override
fn display_override(
    mut window: Single<&mut Window>,
    mut custom_text: Single<&mut Text, With<CustomText>>,
) {
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
    custom_text.0 = text;
}

/// This system toggles scale factor overrides when enter is pressed
fn toggle_override(input: Res<ButtonInput<KeyCode>>, mut window: Single<&mut Window>) {
    if input.just_pressed(KeyCode::Enter) {
        let scale_factor_override = window.resolution.scale_factor_override();
        window
            .resolution
            .set_scale_factor_override(scale_factor_override.xor(Some(1.0)));
    }
}

/// This system changes the scale factor override when up or down is pressed
fn change_scale_factor(input: Res<ButtonInput<KeyCode>>, mut window: Single<&mut Window>) {
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
