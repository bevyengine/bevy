#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

pub mod commands;
/// The basic components of the transform crate
pub mod components;

/// Transform related bundles
pub mod bundles;

/// Transform related traits
pub mod traits;

/// Transform related plugins
pub mod plugins;

/// Helpers related to computing global transforms
pub mod helper;
/// Systems responsible for transform propagation
pub mod systems;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::components::*;

    #[doc(hidden)]
    pub use crate::{
        bundles::TransformBundle, commands::BuildChildrenTransformExt, helper::TransformHelper,
        plugins::TransformPlugin, plugins::TransformSystem, traits::TransformPoint,
    };
}

pub use prelude::{TransformPlugin, TransformPoint, TransformSystem};
