//! A test to confirm that `bevy` doesn't regress on gizmos
//! This is run in CI.

use bevy::prelude::*;

/// A test to confirm that `bevy` doesn't regress on gizmos
/// #14142 is the latest issue where this occurred
pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, |mut gizmos: Gizmos| {
            let rot = Quat::from_array([5.0, 0.0, 0.0, 0.0]); //.normalize();
            gizmos.sphere(Vec3::ZERO, rot, 1.0, Color::srgb(1.0, 0.0, 0.0));
        })
        .run();
}
