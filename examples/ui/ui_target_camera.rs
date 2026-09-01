//! Demonstrates how to use `UiTargetCamera` and camera ordering.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

const BOX_SIZE: f32 = 100.;

/// The color used to clear the viewport while this camera has the lowest order.
#[derive(Component)]
struct CameraClearColor(Color);

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
                    // The viewport will be cleared according to the `ClearColorConfig` of the camera with the lowest order, skipping cameras set to `ClearColorConfig::None`.
                    // If all are set to `ClearColorConfig::None`, no clear color is used.
                    clear_color: if i == 0 {
                        ClearColorConfig::Custom(color.into())
                    } else {
                        ClearColorConfig::None
                    },
                    order: i as isize,
                    ..Default::default()
                },
                CameraClearColor(color.into()),
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
                move |on_pressed: On<PointerPress>,
                      mut label_query: Query<&mut Text>,
                      mut camera_query: Query<(&mut Camera, &CameraClearColor)>| {
                    let Ok(mut label_text) = label_query.get_mut(label_entity) else {
                        return;
                    };
                    let direction = match on_pressed.button {
                        PointerButton::Primary => 1,
                        _ => -1,
                    };
                    // Two cameras with the same order make the order ambiguous, which makes
                    // the render result unpredictable. Instead of a single fixed ±1 step, keep
                    // stepping in the clicked direction until the new order is not used by
                    // any other camera.
                    let Some(mut new_order) = camera_query
                        .get_mut(camera_entity)
                        .ok()
                        .map(|(camera, _)| camera.order)
                    else {
                        return;
                    };
                    new_order += direction;
                    while camera_query
                        .iter()
                        .any(|(camera, _)| camera.order == new_order)
                    {
                        new_order += direction;
                    }
                    if let Ok((mut camera, _)) = camera_query.get_mut(camera_entity) {
                        camera.order = new_order;
                    }
                    // The camera with the lowest order clears the viewport with its own color,
                    // while every other camera is set to `ClearColorConfig::None`, so the clear
                    // color stays correct as camera orders change.
                    let lowest_order = camera_query
                        .iter()
                        .map(|(camera, _)| camera.order)
                        .min()
                        .unwrap();
                    for (mut camera, clear_color) in camera_query.iter_mut() {
                        camera.clear_color = if camera.order == lowest_order {
                            ClearColorConfig::Custom(clear_color.0)
                        } else {
                            ClearColorConfig::None
                        };
                    }
                    label_text.0 = format!("{new_order}");
                },
            )
            .add_child(label_entity);
    }
}
