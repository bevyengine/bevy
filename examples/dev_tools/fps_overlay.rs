//! Showcase how to use and configure FPS overlay.

use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
    prelude::*,
    text::FontSmoothing,
};

struct OverlayColor;

impl OverlayColor {
    const RED: Color = Color::srgb(1.0, 0.0, 0.0);
    const GREEN: Color = Color::srgb(0.0, 1.0, 0.0);
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        // Here we define size of our overlay
                        font_size: 42.0,
                        // If we want, we can use a custom font
                        font: default(),
                        // We could also disable font smoothing,
                        font_smoothing: FontSmoothing::default(),
                        ..default()
                    },
                    // We can also change color of the overlay
                    text_color: OverlayColor::GREEN,
                    // We can also set the refresh interval for the FPS counter
                    refresh_interval: core::time::Duration::from_millis(100),
                    enabled: true,
                    frame_time_graph_config: FrameTimeGraphConfig {
                        enabled: true,
                        // The minimum acceptable fps
                        min_fps: 30.0,
                        // The target fps
                        target_fps: 144.0,
                    },
                },
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, customize_config)
        .run();
}

fn setup(mut commands: Commands) {
    // We need to spawn a camera (2d or 3d) to see the overlay
    commands.spawn(Camera2d);

    // Instruction text

    commands.spawn((
        Text::new(concat!(
            "Press 1 to toggle the overlay color.\n",
            "Press 2 to decrease the overlay size.\n",
            "Press 3 to increase the overlay size.\n",
            "Press 4 to toggle the text visibility.\n",
            "Press 5 to toggle the frame time graph."
        )),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn customize_config(input: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<FpsOverlayConfig>) {
    if input.just_pressed(KeyCode::Digit1) {
        // Changing resource will affect overlay
        if overlay.text_color == OverlayColor::GREEN {
            overlay.text_color = OverlayColor::RED;
        } else {
            overlay.text_color = OverlayColor::GREEN;
        }
    }
    if input.just_pressed(KeyCode::Digit2) {
        overlay.text_config.font_size -= 2.0;
    }
    if input.just_pressed(KeyCode::Digit3) {
        overlay.text_config.font_size += 2.0;
    }
    if input.just_pressed(KeyCode::Digit4) {
        overlay.enabled = !overlay.enabled;
    }
    if input.just_released(KeyCode::Digit5) {
        overlay.frame_time_graph_config.enabled = !overlay.frame_time_graph_config.enabled;
    }
}
