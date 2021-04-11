use bevy_window::WindowId;

use crate::texture::TextureFormat;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SwapChainDescriptor {
    pub window_id: WindowId,
    /// The texture format of the swap chain. The only formats that are guaranteed are
    /// `Bgra8Unorm` and `Bgra8UnormSrgb`
    pub format: TextureFormat,
    /// Width of the swap chain. Must be the same size as the surface.
    pub width: u32,
    /// Height of the swap chain. Must be the same size as the surface.
    pub height: u32,
    pub vsync: bool,
}
