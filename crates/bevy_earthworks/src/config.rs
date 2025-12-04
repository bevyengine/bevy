//! Configuration for the Earthworks plugin.

use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;

/// Global configuration for the Earthworks plugin.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct EarthworksConfig {
    /// Size of a single voxel in world units (meters).
    /// Default: 0.3048 (1 foot)
    pub voxel_size: f32,

    /// Number of voxels per chunk dimension.
    /// Default: 16 (16x16x16 voxels per chunk)
    pub chunk_size: u32,

    /// Maximum number of chunks to mesh per frame.
    /// Default: 2
    pub max_meshes_per_frame: u32,

    /// Whether to show debug gizmos for work envelopes.
    /// Default: false
    pub show_work_envelopes: bool,

    /// Whether to show debug gizmos for chunk bounds.
    /// Default: false
    pub show_chunk_bounds: bool,

    /// Playback speed multiplier.
    /// Default: 1.0
    pub playback_speed: f32,

    /// Whether the simulation is currently playing.
    /// Default: false
    pub is_playing: bool,

    /// Whether to loop playback when reaching the end.
    /// Default: false
    pub loop_playback: bool,

    /// Whether to show the timeline/playback UI.
    /// Set to false when embedding earthworks in another application with its own UI.
    /// Default: true
    pub show_ui: bool,

    /// Whether to show the Zyns HUD overlay.
    /// Default: true
    pub show_zyns_hud: bool,

    /// Whether to enable achievement particle effects.
    /// Default: true
    pub enable_achievement_effects: bool,
}

impl Default for EarthworksConfig {
    fn default() -> Self {
        Self {
            voxel_size: 0.3048, // 1 foot in meters
            chunk_size: 16,
            max_meshes_per_frame: 2,
            show_work_envelopes: false,
            show_chunk_bounds: false,
            playback_speed: 1.0,
            is_playing: false,
            loop_playback: false,
            show_ui: true,
            show_zyns_hud: true,
            enable_achievement_effects: true,
        }
    }
}
