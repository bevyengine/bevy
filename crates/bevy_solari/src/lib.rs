//! Provides raytraced lighting.

pub mod scene;

/// The solari prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use crate::scene::RaytracingMesh3d;
}

use bevy_app::{App, Plugin};
use scene::SolariScenePlugin;

pub struct SolariPlugin;

impl Plugin for SolariPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SolariScenePlugin);
    }
}
