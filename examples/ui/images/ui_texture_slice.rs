//! This example illustrates how to create buttons with their textures sliced
//! and kept in proportion instead of being stretched by the button dimensions.
//!
//! It uses the `bevy_ui_widgets` headless [`Button`] widget. The button's hover and press
//! state is tracked by the `Hovered` and `Pressed` components; a `Changed`-based system
//! updates the sliced image's tint and label to match.

use bevy::{
    color::palettes::css::{GOLD, ORANGE},
    picking::hover::Hovered,
    prelude::*,
    ui::{widget::NodeImageMode, Pressed},
    ui_widgets::Button,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (button_system, button_system_removals))
        .run();
}

/// Updates each button's sliced image tint and label whenever its `Pressed` or `Hovered`
/// state changes.
fn button_system(
    mut buttons: Query<
        (Has<Pressed>, &Hovered, &mut ImageNode, &Children),
        (Or<(Changed<Pressed>, Changed<Hovered>)>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (pressed, hovered, mut image, children) in &mut buttons {
        set_button_style(
            pressed,
            hovered.get(),
            &mut image,
            children,
            &mut text_query,
        );
    }
}

/// Supplementary system to detect the removal of `Pressed`, which `Changed` does not report.
fn button_system_removals(
    mut buttons: Query<(Has<Pressed>, &Hovered, &mut ImageNode, &Children), With<Button>>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut text_query: Query<&mut Text>,
) {
    for entity in removed_pressed.read() {
        if let Ok((pressed, hovered, mut image, children)) = buttons.get_mut(entity) {
            set_button_style(
                pressed,
                hovered.get(),
                &mut image,
                children,
                &mut text_query,
            );
        }
    }
}

/// Shared styling logic: pressed takes precedence over hover, matching the original example.
fn set_button_style(
    pressed: bool,
    hovered: bool,
    image: &mut ImageNode,
    children: &Children,
    text_query: &mut Query<&mut Text>,
) {
    let mut text = text_query.get_mut(children[0]).unwrap();
    match (pressed, hovered) {
        (true, _) => {
            **text = "Press".to_string();
            image.color = GOLD.into();
        }
        (false, true) => {
            **text = "Hover".to_string();
            image.color = ORANGE.into();
        }
        (false, false) => {
            **text = "Button".to_string();
            image.color = Color::WHITE;
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load("textures/fantasy_ui_borders/panel-border-010.png");

    let slicer = TextureSlicer {
        border: BorderRect::all(22.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };
    // ui camera
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            for [w, h] in [[150.0, 150.0], [300.0, 150.0], [150.0, 300.0]] {
                parent.spawn((
                    Button,
                    // Required so the button tracks hover state (used by `button_system`).
                    Hovered::default(),
                    ImageNode {
                        image: image.clone(),
                        image_mode: NodeImageMode::Sliced(slicer.clone()),
                        ..default()
                    },
                    Node {
                        width: px(w),
                        height: px(h),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        margin: UiRect::all(px(20)),
                        ..default()
                    },
                    children![(
                        Text::new("Button"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(33.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    )],
                ));
            }
        });
}
