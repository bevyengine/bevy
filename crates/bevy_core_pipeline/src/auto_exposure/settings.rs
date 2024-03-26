use super::compensation_curve::AutoExposureCompensationCurve;
use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, query::QueryItem, reflect::ReflectComponent};
use bevy_reflect::Reflect;
use bevy_render::{extract_component::ExtractComponent, texture::Image};
use bevy_utils::default;

/// Component that enables auto exposure for a camera.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct AutoExposureSettings {
    /// The minimum exposure value for the camera.
    pub min: f32,
    /// The maximum exposure value for the camera.
    pub max: f32,
    /// The percentage of darkest pixels to ignore when metering.
    pub low_percent: u32,
    /// The percentage of brightest pixels to ignore when metering.
    pub high_percent: u32,
    /// The speed at which the exposure adapts from dark to bright scenes, in F-stops per second.
    pub speed_brighten: f32,
    /// The speed at which the exposure adapts from bright to dark scenes, in F-stops per second.
    pub speed_darken: f32,
    /// The mask to apply when metering. Bright spots on the mask will contribute more to the
    /// metering, and dark spots will contribute less.
    pub metering_mask: Handle<Image>,
    /// Exposure compensation curve to apply after metering.
    pub compensation_curve: Handle<AutoExposureCompensationCurve>,
}

impl Default for AutoExposureSettings {
    fn default() -> Self {
        Self {
            min: -8.0,
            max: 8.0,
            low_percent: 60,
            high_percent: 95,
            speed_brighten: 3.0,
            speed_darken: 1.0,
            metering_mask: default(),
            compensation_curve: default(),
        }
    }
}

impl ExtractComponent for AutoExposureSettings {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(item.clone())
    }
}
