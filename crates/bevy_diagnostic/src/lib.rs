// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate provides a straightforward solution for integrating diagnostics in the [Bevy game engine](https://bevyengine.org/).
//! It allows users to easily add diagnostic functionality to their Bevy applications, enhancing
//! their ability to monitor and optimize their game's.

mod diagnostic;
mod entity_count_diagnostics_plugin;
mod frame_time_diagnostics_plugin;
mod log_diagnostics_plugin;
#[cfg(feature = "sysinfo_plugin")]
mod system_information_diagnostics_plugin;

pub use diagnostic::*;

pub use entity_count_diagnostics_plugin::EntityCountDiagnosticsPlugin;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;
pub use log_diagnostics_plugin::LogDiagnosticsPlugin;
#[cfg(feature = "sysinfo_plugin")]
pub use system_information_diagnostics_plugin::{SystemInfo, SystemInformationDiagnosticsPlugin};

use bevy_app::prelude::*;

/// Adds core diagnostics resources to an App.
#[derive(Default)]
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiagnosticsStore>();

        #[cfg(feature = "sysinfo_plugin")]
        app.init_resource::<system_information_diagnostics_plugin::SystemInfo>();
    }
}

/// Default max history length for new diagnostics.
pub const DEFAULT_MAX_HISTORY_LENGTH: usize = 120;
