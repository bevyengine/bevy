#![cfg(feature = "custom_window_icon")]

use bevy_image::{Image, IntoDynamicImageError};
use bevy_window::WindowIconSource;
use winit::window::BadIcon;
pub use winit::window::Icon;

#[allow(
    clippy::allow_attributes,
    reason = "The unused variants detection is tricky so instead of `expect` we use `allow` here."
)]
#[allow(
    dead_code,
    reason = "Bevy only supports custom window icons for Windows at this time. The variants that are dead code here depend on the platform."
)]
pub(crate) enum CreateWinitWindowIconError {
    RgbaConversionFailed(IntoDynamicImageError),
    BadIcon(BadIcon),
    PlatformNotSupported,
}
impl core::fmt::Display for CreateWinitWindowIconError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CreateWinitWindowIconError::RgbaConversionFailed(e) => {
                write!(f, "Failed to convert image to RGBA: {}", e)
            }
            CreateWinitWindowIconError::BadIcon(e) => {
                write!(f, "Failed to create icon from RGBA data: {}", e)
            }
            CreateWinitWindowIconError::PlatformNotSupported => {
                write!(f, "Inheriting window icon from process executable is not supported on this platform.")
            }
        }
    }
}

pub(crate) fn create_winit_window_icon_from_bevy_image(
    image: Image,
) -> Result<Icon, CreateWinitWindowIconError> {
    // Convert to rgba image
    let rgba_image = match image.try_into_dynamic() {
        Ok(dynamic_image) => {
            // winit icon expects 32bpp RGBA data
            dynamic_image.into_rgba8()
        }
        Err(error) => {
            return Err(CreateWinitWindowIconError::RgbaConversionFailed(error));
        }
    };

    // Convert to winit image
    let width = rgba_image.width();
    let height = rgba_image.height();
    match Icon::from_rgba(rgba_image.into_raw(), width, height) {
        Ok(icon) => Ok(icon),
        Err(error) => Err(CreateWinitWindowIconError::BadIcon(error)),
    }
}

pub(crate) fn create_winit_window_icon_using_platform_mechanism(
    #[cfg_attr(
        not(target_os = "windows"),
        expect(
            unused_variables,
            reason = "Bevy only supports custom window icons for Windows at this time. There is a zero-variant WindowIconSource enum on unsupported platforms."
        )
    )]
    window_icon_source: &WindowIconSource,
) -> Result<Icon, CreateWinitWindowIconError> {
    #[cfg(target_os = "windows")]
    {
        use winit::{dpi::PhysicalSize, platform::windows::IconExtWindows};

        match window_icon_source {
            WindowIconSource::Path { path, size } => {
                IconExtWindows::from_path(path, size.map(|(w, h)| PhysicalSize::new(w, h)))
                    .map_err(CreateWinitWindowIconError::BadIcon)
            }
            WindowIconSource::ResourceOrdinal(i) => {
                IconExtWindows::from_resource(*i, None).map_err(CreateWinitWindowIconError::BadIcon)
            }
            WindowIconSource::ResourceName(name) => IconExtWindows::from_resource_name(name, None)
                .map_err(CreateWinitWindowIconError::BadIcon),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(CreateWinitWindowIconError::PlatformNotSupported)
    }
}
