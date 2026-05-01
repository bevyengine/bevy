use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// Scale factor for ui node rasterization.
///
/// Add it to an entity with [`Node`] to multiply the raster resolution of rasterized parts
/// of the node and it's **descendants**, such as text.
///
/// Does not affect the logical or physical size of the node.
#[derive(Component, Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct UiRasterScale(pub f32);

impl UiRasterScale {
    pub const DEFAULT: Self = Self(1.0);
}

impl Default for UiRasterScale {
    fn default() -> Self {
        Self::DEFAULT
    }
}
