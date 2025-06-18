//! This module provides panic handlers for [Bevy](https://bevy.org)
//! apps, and automatically configures platform specifics (i.e. Wasm or Android).
//!
//! By default, the [`PanicHandlerPlugin`] from this crate is included in Bevy's `DefaultPlugins`.
//!
//! For more fine-tuned control over panic behavior, disable the [`PanicHandlerPlugin`] or
//! `DefaultPlugins` during app initialization.

use crate::{App, Plugin};

/// Adds sensible panic handlers to Apps. This plugin is part of the `DefaultPlugins`. Adding
/// this plugin will setup a panic hook appropriate to your target platform:
/// * On Wasm, uses [`console_error_panic_hook`](https://crates.io/crates/console_error_panic_hook), logging
///   to the browser console.
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
        #[cfg(feature = "std")]
        {
            static SET_HOOK: std::sync::Once = std::sync::Once::new();
            SET_HOOK.call_once(|| {
                cfg_if::cfg_if! {
                    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
                        // This provides better panic handling in JS engines (displays the panic message and improves the backtrace).
                        std::panic::set_hook(alloc::boxed::Box::new(console_error_panic_hook::hook));
                    } else if #[cfg(feature = "error_panic_hook")] {
                        let current_hook = std::panic::take_hook();
                        std::panic::set_hook(alloc::boxed::Box::new(
                            bevy_ecs::error::bevy_error_panic_hook(current_hook),
                        ));
                    }
                    // Otherwise use the default target panic hook - Do nothing.
                }
            });
        }
    }
}
