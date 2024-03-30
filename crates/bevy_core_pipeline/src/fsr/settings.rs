use crate::{DepthPrepass, MotionVectorPrepass};
use bevy_ecs::{bundle::Bundle, component::Component, reflect::ReflectComponent};
use bevy_reflect::Reflect;
use bevy_render::camera::{MipBias, TemporalJitter};

#[derive(Bundle, Default)]
pub struct FsrBundle {
    pub settings: FsrSettings,
    pub jitter: TemporalJitter,
    pub mip_bias: MipBias,
    pub depth_prepass: DepthPrepass,
    pub motion_vector_prepass: MotionVectorPrepass,
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct FsrSettings {
    pub quality_mode: FsrQualityMode,
    pub reset: bool,
}

impl Default for FsrSettings {
    fn default() -> Self {
        Self {
            quality_mode: FsrQualityMode::Balanced,
            reset: false,
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug)]
pub enum FsrQualityMode {
    /// No upscaling, just antialiasing.
    Native,
    /// Upscale by 1.5x.
    Quality,
    /// Upscale by 1.7x.
    Balanced,
    /// Upscale by 2.0x.
    Peformance,
    /// Upscale by 3.0x.
    UltraPerformance,
}
