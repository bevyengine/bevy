use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::{camera::CameraChainPosition, color::Color, extract_resource::ExtractResource};
use serde::{Deserialize, Serialize};

#[derive(Reflect, Serialize, Deserialize, Clone, Debug, Default)]
#[reflect(Serialize, Deserialize)]
pub enum ClearColorConfig {
    #[default]
    /// For a single camera, clears the camera to the [`ClearColor`]. For multi-camera setups
    /// the default behaviour depends on current camera position, see [`CameraOutputMode`].
    ///
    /// [`CameraOutputMode`]: bevy_render::camera::CameraOutputMode
    Default,
    Custom(Color),
    None,
}

impl ClearColorConfig {
    pub fn load_op<V: From<bevy_render::color::Color>>(
        &self,
        chain_position: CameraChainPosition,
        default_color: &Color,
    ) -> wgpu::LoadOp<V> {
        match self {
            // default behaviour depends on current camera position, see [`CameraOutputMode`].
            ClearColorConfig::Default => match chain_position {
                // first camera in a chain uses the default clear color
                CameraChainPosition::First => wgpu::LoadOp::Clear((*default_color).into()),
                // cameras that build on previous results will load those results
                CameraChainPosition::NotFirst => wgpu::LoadOp::Load,
                // cameras that are first after a flush clear to transparent so they can be blended with previous results
                CameraChainPosition::FirstAfterFlush => wgpu::LoadOp::Clear(Color::NONE.into()),
            },
            ClearColorConfig::Custom(color) => wgpu::LoadOp::Clear((*color).into()),
            ClearColorConfig::None => wgpu::LoadOp::Load,
        }
    }
}

/// A [`Resource`] that stores the color that is used to clear the screen between frames.
///
/// This color appears as the "background" color for simple apps,
/// when there are portions of the screen with nothing rendered.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::rgb(0.4, 0.4, 0.4))
    }
}
