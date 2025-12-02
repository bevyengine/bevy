//! Visual effects for earthworks simulation.
//!
//! This module provides particle effects and visual feedback for:
//! - Excavation dust clouds
//! - Material dumps
//! - Machine movement trails

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Plugin for visual effects.
pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, _app: &mut App) {
        // Effects plugin is a placeholder for future particle systems
    }
}

/// Configuration for visual effects.
#[derive(Resource, Clone, Debug)]
pub struct EffectsConfig {
    /// Whether effects are enabled.
    pub enabled: bool,
    /// Dust particle count multiplier.
    pub dust_multiplier: f32,
    /// Trail length.
    pub trail_length: u32,
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dust_multiplier: 1.0,
            trail_length: 100,
        }
    }
}
