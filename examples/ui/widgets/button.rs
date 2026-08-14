//! This example illustrates how to create a button using the `bevy_ui_widgets` widget set:
//! a headless `Button` that fires an `Activate` event when clicked.
//!
//! The `bevy_ui_widgets` widgets are behavior-only — the `Button` tracks its own pressed state
//! and detects clicks, but comes with no styling. We supply the look ourselves.
//!
//! ## What's next?
//!
//! - Keyboard & accessibility: add `bevy::input_focus::tab_navigation::TabNavigationPlugin`, wrap
//!   your UI in a `TabGroup`, and give the button a `TabIndex`. It can then be focused with Tab and
//!   activated with Enter/Space, firing the same `Activate` event. (`Button` already reports itself
//!   to accessibility tools via the `AccessibilityNode` it requires.)
//!
//! - Disabling a button: insert the `bevy::ui::InteractionDisabled` marker to stop it from
//!   activating, and branch on `Has<InteractionDisabled>` in `update_button_appearance` to grey it
//!   out. See the `standard_widgets` and `standard_widgets_observers` examples for that pattern.
//!
//! - Activate on press instead of on release: add the `ActivateOnPress` marker (useful for things
//!   like menu buttons that should fire the instant they're pressed).
//!
//! - Reacting via observers instead of a polling system: rather than updating appearance every
//!   frame, you can observe component changes (e.g. `On<Insert, Pressed>`). The
//!   `standard_widgets_observers` example demonstrates this approach.

use bevy::{
    color::palettes::basic::*,
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{observe, Activate, Button},
};

fn main() {
    App::new()
        // `DefaultPlugins` already includes the `bevy_ui_widgets` plugins, so the `Button`
        // widget's behavior (and its `Activate` event) works out of the box.
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // Update the button's appearance every frame based on its current state.
        .add_systems(Update, update_button_appearance)
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2d);

    // A full-screen container that centers the button.
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            button(&assets),
            // React to the button being clicked. `Button` fires an `Activate` event on a
            // completed click, so we attach an observer for it right here on the button entity.
            //
            // Try it: this is where you'd run your own logic (start a game, open a menu, etc.).
            observe(|_activate: On<Activate>| {
                info!("Button clicked!");
            }),
        )],
    ));
}

fn button(asset_server: &AssetServer) -> impl Bundle {
    (
        // The headless button widget. It handles pointer/keyboard input and pressed-state
        // tracking for us; we only provide the look below.
        Button,
        // `Hovered` is used by the picking backend to track if the pointer is over the button.
        Hovered::default(),
        Node {
            width: px(150),
            height: px(65),
            border: UiRect::all(px(5)),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BorderColor::all(Color::BLACK),
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new("Button"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: FontSize::Px(33.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    )
}

/// Restyle the button and update its label to reflect its current state.
///
/// The `Button` widget maintains a `Pressed` component while the button is held down, and the
/// picking backend keeps `Hovered` up to date. We simply read those each frame and pick a look.
fn update_button_appearance(
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<Button>,
    >,
    mut text_query: Query<&mut Text>,
) {
    for (hovered, pressed, mut color, mut border_color, children) in &mut buttons {
        let Ok(mut text) = text_query.get_mut(children[0]) else {
            continue;
        };

        match (hovered.get(), pressed) {
            // Pressed (and, since you can only press what you're hovering, also hovered).
            (_, true) => {
                **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.set_all(RED);
            }
            // Hovered but not pressed.
            (true, false) => {
                **text = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.set_all(WHITE);
            }
            // Neither hovered nor pressed.
            (false, false) => {
                **text = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.set_all(BLACK);
            }
        }
    }
}
