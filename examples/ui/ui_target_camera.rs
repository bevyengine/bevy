//! Demonstrates how to use `UiTargetCamera` and camera ordering.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::log::LogPlugin;
use bevy::log::DEFAULT_FILTER;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            // Disable camera order ambiguity warnings
            filter: format!("{DEFAULT_FILTER},bevy_render::camera=off"),
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .run();
}

const BOX_SIZE: f32 = 100.;

fn setup(mut commands: Commands) {
    // Root UI node displaying instructions.
    // Has no `UiTargetCamera`; the highest-order camera rendering to the primary window will be chosen automatically.
    commands.spawn((
        Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                justify_content: JustifyContent::Center,
                bottom: px(2. * BOX_SIZE),
                ..default()
            },
            Text::new("Each box is rendered by a different camera\n* left-click: increase the camera's order\n* right-click: decrease the camera's order")
        ));

    for (i, color) in [RED, GREEN, BLUE].into_iter().enumerate() {
        let camera_entity = commands
            .spawn((
                // Ordering behavior is the same using `Camera3d`.
                Camera2d,
                Camera {
                    // The viewport will be cleared according to the `ClearColorConfig` of the camera with the lowest order, skipping cameras set to `ClearColorConfig::NONE`.
                    // If all are set to `ClearColorConfig::NONE`, no clear color is used.
                    clear_color: ClearColorConfig::Custom(color.into()),
                    order: i as isize,
                    ..Default::default()
                },
            ))
            .id();

        // Label each box with the order of its camera target
        let label_entity = commands
            .spawn((
                Text(format!("{i}")),
                TextFont::from_font_size(50.),
                TextColor(color.into()),
            ))
            .id();

        commands
            .spawn((
                Node {
                    align_self: AlignSelf::Center,
                    justify_self: JustifySelf::Center,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    left: px(0.67 * BOX_SIZE * (i as f32 - 1.)),
                    top: px(0.67 * BOX_SIZE * (i as f32 - 1.)),
                    width: px(BOX_SIZE),
                    height: px(BOX_SIZE),
                    border: px(0.1 * BOX_SIZE).into(),
                    ..default()
                },
                // Bevy UI doesn't support `RenderLayers`. Each UI layout can only have one render target, selected using `UiTargetCamera`.
                UiTargetCamera(camera_entity),
                BackgroundColor(Color::BLACK),
                BorderColor::all(YELLOW),
            ))
            .observe(
                move |on_pressed: On<Pointer<Press>>,
                      mut label_query: Query<&mut Text>,
                      mut camera_query: Query<&mut Camera>| {
                    let Ok(mut label_text) = label_query.get_mut(label_entity) else {
                        return;
                    };
                    let Ok(mut camera) = camera_query.get_mut(camera_entity) else {
                        return;
                    };
                    camera.order += match on_pressed.button {
                        PointerButton::Primary => 1,
                        _ => -1,
                    };
                    label_text.0 = format!("{}", camera.order);
                },
            )
            .add_child(label_entity);
    }
}
