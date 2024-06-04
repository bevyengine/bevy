use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_math::{IVec2, UVec2};
use bevy_reflect::Reflect;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Represents a monitor attached to the system, which can be used to create windows.
///
/// This component is synchronized with `winit` through `bevy_winit`, but is effectively
/// read-only as `winit` does not support changing monitor properties.
#[derive(Component, Debug, Clone, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Component)]
pub struct Monitor {
    /// The name of the monitor
    pub name: Option<String>,
    /// The height of the monitor in physical pixels
    pub physical_height: u32,
    /// The width of the monitor in physical pixels
    pub physical_width: u32,
    /// The position of the monitor in physical pixels
    pub physical_position: IVec2,
    /// The refresh rate of the monitor in millihertz
    pub refresh_rate_millihertz: Option<u32>,
    /// The scale factor of the monitor
    pub scale_factor: f64,
    /// The video modes that the monitor supports
    pub video_modes: Vec<VideoMode>,
}

/// A marker component for the primary monitor
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct PrimaryMonitor;

impl Monitor {
    /// Returns the physical size of the monitor in pixels
    pub fn physical_size(&self) -> UVec2 {
        UVec2::new(self.physical_width, self.physical_height)
    }
}

/// Represents a video mode that a monitor supports
#[derive(Debug, Clone, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct VideoMode {
    /// The resolution of the video mode
    pub physical_size: UVec2,
    /// The bit depth of the video mode
    pub bit_depth: u16,
    /// The refresh rate in millihertz
    pub refresh_rate_millihertz: u32,
}
