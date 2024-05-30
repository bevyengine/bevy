#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

/// The basic components of the transform crate
pub mod components;

#[cfg(feature = "bevy-support")]
pub mod commands;

/// Transform related bundles
#[cfg(feature = "bevy-support")]
pub mod bundles;

/// Transform related bundles
#[cfg(feature = "bevy-support")]
pub mod traits;

/// Transform related plugins
#[cfg(feature = "bevy-support")]
pub mod plugins;

#[cfg(feature = "bevy-support")]
pub mod helper;

/// Systems responsible for transform propagation
#[cfg(feature = "bevy-support")]
pub mod systems;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::components::Transform;

    #[cfg(feature = "bevy-support")]
    #[doc(hidden)]
    pub use crate::{
        bundles::TransformBundle, commands::BuildChildrenTransformExt, components::*,
        helper::TransformHelper, plugins::TransformPlugin, traits::TransformPoint,
    };
}

#[cfg(feature = "bevy-support")]
use prelude::{GlobalTransform, Transform};
