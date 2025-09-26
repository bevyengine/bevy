use std::time::{SystemTime, UNIX_EPOCH};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_input::{common_conditions::input_just_pressed, keyboard::KeyCode};
use bevy_render::view::screenshot::{save_to_disk, Screenshot};
use bevy_window::{PrimaryWindow, Window};

/// File format the screenshot will be saved in
#[derive(Clone, Copy)]
pub enum ScreenshotFormat {
    /// JPEG format
    Jpeg,
    /// PNG format
    Png,
    /// BMP format
    Bmp,
}

/// Add this plugin to your app to enable easy screenshotting.
///
/// Add this plugin, press the key, and you have a screenshot 🎉
pub struct EasyScreenshotPlugin {
    /// Key that will trigger a screenshot
    pub trigger: KeyCode,
    /// Format of the screenshot
    ///
    /// The corresponding image format must be supported by bevy renderer
    pub format: ScreenshotFormat,
}

impl Default for EasyScreenshotPlugin {
    fn default() -> Self {
        EasyScreenshotPlugin {
            trigger: KeyCode::PrintScreen,
            format: ScreenshotFormat::Png,
        }
    }
}

impl Plugin for EasyScreenshotPlugin {
    fn build(&self, app: &mut App) {
        let format = self.format;
        app.add_systems(
            Update,
            (move |mut commands: Commands, window: Single<&Window, With<PrimaryWindow>>| {
                let since_the_epoch = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time should go forward");

                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(format!(
                        "{}-{}.{}",
                        window.title,
                        since_the_epoch.as_millis(),
                        match format {
                            ScreenshotFormat::Jpeg => "jpg",
                            ScreenshotFormat::Png => "png",
                            ScreenshotFormat::Bmp => "bmp",
                        }
                    )));
            })
            .run_if(input_just_pressed(self.trigger)),
        );
    }
}
