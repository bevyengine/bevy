#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! This crate provides additional utilities for the [Bevy game engine](https://bevy.org),
//! focused on improving developer experience.

extern crate alloc;

#[cfg(feature = "bevy_ci_testing")]
pub mod ci_testing;

pub mod diagnostics_overlay;
mod easy_screenshot;
pub mod fps_overlay;
pub mod frame_time_graph;

pub mod picking_debug;

#[cfg(feature = "schedule_data")]
pub mod schedule_data;

pub mod states;

pub use easy_screenshot::*;

pub mod render_debug;
