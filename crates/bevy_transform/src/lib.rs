#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "bevy_ecs")]
pub mod commands;
/// The basic components of the transform crate
pub mod components;

/// Transform related traits
pub mod traits;

/// Transform related plugins
#[cfg(all(feature = "bevy_ecs", feature = "bevy_app"))]
pub mod plugins;

/// [`GlobalTransform`]: components::GlobalTransform
/// Helpers related to computing global transforms
#[cfg(feature = "bevy_ecs")]
pub mod helper;
/// Systems responsible for transform propagation
#[cfg(feature = "bevy_ecs")]
pub mod systems;

/// The transform prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, traits::TransformPoint};

    #[cfg(feature = "bevy_ecs")]
    #[doc(hidden)]
    pub use crate::{
        commands::BuildChildrenTransformExt,
        helper::TransformHelper,
        systems::{StaticTransformOptimizations, TransformSystems},
    };

    #[cfg(all(feature = "bevy_ecs", feature = "bevy_app"))]
    #[doc(hidden)]
    pub use crate::plugins::TransformPlugin;
}

pub use traits::TransformPoint;

#[cfg(feature = "bevy_ecs")]
pub use prelude::{StaticTransformOptimizations, TransformSystems};

#[cfg(feature = "bevy_app")]
pub use prelude::TransformPlugin;
