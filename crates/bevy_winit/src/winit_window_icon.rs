#![cfg(feature = "custom_window_icon")]

use bevy_image::Image;
use tracing::error;

pub(crate) fn create_winit_window_icon_from_bevy_image(
    image: Image,
) -> Option<winit::window::Icon> {
    // Convert to rgba image
    let rgba_image = match image.try_into_dynamic() {
        Ok(dynamic_image) => {
            // winit icon expects 32bpp RGBA data
            dynamic_image.into_rgba8()
        }
        Err(error) => {
            error!(
                ?error,
                "Could not create winit window icon from Bevy assets: failed to convert image to RGBA",
            );
            return None;
        }
    };

    // Convert to winit image
    let width = rgba_image.width();
    let height = rgba_image.height();
    match winit::window::Icon::from_rgba(rgba_image.into_raw(), width, height) {
        Ok(icon) => Some(icon),
        Err(error) => {
            error!(
                ?error,
                "Could not create winit window icon from Bevy assets: failed to construct winit window icon from RGBA buffer",
            );
            None
        }
    }
}
