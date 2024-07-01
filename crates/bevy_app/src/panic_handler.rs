//! This module provides panic handlers for [Bevy](https://bevyengine.org)
//! apps, and automatically configures platform specifics (i.e. WASM or Android).
//!
//! By default, the [`PanicHandlerPlugin`] from this crate is included in Bevy's `DefaultPlugins`.
//!
//! For more fine-tuned control over panic behavior, disable the [`PanicHandlerPlugin`] or
//! `DefaultPlugins` during app initialization.

use crate::App;
use crate::Plugin;

/// Adds sensible panic handlers to Apps. This plugin is part of the `DefaultPlugins`. Adding
/// this plugin will setup a panic hook appropriate to your target platform:
/// * On WASM, uses [`console_error_panic_hook`](https://crates.io/crates/console_error_panic_hook), logging
///     to the browser console.
/// * Other platforms are currently not setup.
///
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as MinimalPlugins, PluginGroup, PanicHandlerPlugin};
/// fn main() {
///     App::new()
///         .add_plugins(MinimalPlugins)
///         .add_plugins(PanicHandlerPlugin)
///         .run();
/// }
/// ```
///
/// If you want to setup your own panic handler, you should disable this
/// plugin from `DefaultPlugins`:
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, PanicHandlerPlugin};
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.build().disable::<PanicHandlerPlugin>())
///         .run();
/// }
/// ```
#[derive(Default)]
pub struct PanicHandlerPlugin;

impl Plugin for PanicHandlerPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            console_error_panic_hook::set_once();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Use the default target panic hook - Do nothing.
        }
    }
}
