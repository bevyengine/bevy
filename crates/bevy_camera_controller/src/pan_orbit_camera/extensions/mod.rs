//! Extensions to the base camera controller.

pub mod dolly_zoom;
pub mod look_to;

#[cfg(feature = "extension_anchor_indicator")]
pub mod anchor_indicator;
#[cfg(feature = "extension_independent_skybox")]
pub mod independent_skybox;
