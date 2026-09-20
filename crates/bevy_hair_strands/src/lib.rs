#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Strand-based hair rendering for Bevy.
//!
//! A [`HairStrands`] asset is a list of polylines, one per strand. Entities
//! that carry a [`HairStrands3d`] component and a
//! [`MeshMaterial3d<HairMaterial>`](bevy_pbr::MeshMaterial3d) have a ribbon
//! [`Mesh`](bevy_mesh::Mesh) generated for them automatically. On the GPU each
//! ribbon is widened to face the camera and shaded with an anisotropic
//! Kajiya-Kay model that reacts to every light, shadow map and fog volume in
//! the scene like `StandardMaterial` does. Far away, only as many strands are
//! drawn as keep each about a pixel wide ([`HairMaterial::min_pixel_width`]),
//! the survivors widened to cover for the rest.
//!
//! ```ignore
//! use bevy_hair_strands::{HairMaterial, HairStrand, HairStrands, HairStrands3d};
//!
//! fn setup(
//!     mut commands: Commands,
//!     mut strands: ResMut<Assets<HairStrands>>,
//!     mut materials: ResMut<Assets<HairMaterial>>,
//! ) {
//!     let hair = strands.add(HairStrands::from_iter([
//!         HairStrand::new([Vec3::ZERO, Vec3::Y * 0.5, Vec3::new(0.2, 0.9, 0.0)]),
//!     ]));
//!     commands.spawn((
//!         HairStrands3d(hair),
//!         MeshMaterial3d(materials.add(HairMaterial::default())),
//!     ));
//! }
//! ```

mod material;
mod strands;

pub use material::{HairMaterial, HairMaterialUniform, MIN_KEEP_FRACTION};
pub use strands::{
    update_hair_strand_bounds, update_hair_strand_meshes, HairStrand, HairStrands, HairStrands3d,
    HairStrandsBoundsPadding, ATTRIBUTE_HAIR_PARAMS, ATTRIBUTE_HAIR_TANGENT,
};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{embedded_asset, AssetApp, AssetEventSystems};
use bevy_camera::visibility::VisibilitySystems;
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bevy_pbr::MaterialPlugin;
use bevy_shader::load_shader_library;

/// The `bevy_hair_strands` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{HairMaterial, HairStrand, HairStrands, HairStrands3d, HairStrandsPlugin};
}

/// System set in which [`HairStrands3d`] entities get their ribbon meshes and
/// bounds rebuilt.
///
/// Runs in [`PostUpdate`] before [`AssetEventSystems`] so that a generated
/// [`Mesh`](bevy_mesh::Mesh) is announced to the renderer in the same frame
/// its entity is, and before visibility bounds are checked. A change to a
/// [`HairStrands`] or [`HairMaterial`] asset is therefore picked up on the
/// following frame.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HairStrandsSystems;

/// Registers the [`HairStrands`] asset, the [`HairMaterial`] and the system
/// that turns strands into renderable ribbons.
#[derive(Default)]
pub struct HairStrandsPlugin;

impl Plugin for HairStrandsPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "hair_functions.wesl");
        embedded_asset!(app, "hair.wesl");
        embedded_asset!(app, "hair_prepass.wesl");

        app.init_asset::<HairStrands>()
            .register_asset_reflect::<HairStrands>()
            .register_type::<HairStrand>()
            .register_type::<HairStrands3d>()
            .register_type::<HairStrandsBoundsPadding>()
            .register_type::<HairMaterial>()
            .add_plugins(MaterialPlugin::<HairMaterial>::default())
            .configure_sets(
                PostUpdate,
                HairStrandsSystems
                    .before(AssetEventSystems)
                    .before(VisibilitySystems::CalculateBounds),
            )
            .add_systems(
                PostUpdate,
                (update_hair_strand_meshes, update_hair_strand_bounds)
                    .chain()
                    .in_set(HairStrandsSystems),
            );
    }
}
