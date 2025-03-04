use core::sync::atomic::{AtomicBool, Ordering};

use bevy_ecs::event::EventWriter;

use crate::{App, AppExit, Plugin, Update};

pub use ctrlc;

/// Indicates that all [`App`]'s should exit.
static SHOULD_EXIT: AtomicBool = AtomicBool::new(false);

/// Gracefully handles `Ctrl+C` by emitting a [`AppExit`] event. This plugin is part of the `DefaultPlugins`.
///
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as MinimalPlugins, PluginGroup, TerminalCtrlCHandlerPlugin};
/// fn main() {
///     App::new()
///         .add_plugins(MinimalPlugins)
///         .add_plugins(TerminalCtrlCHandlerPlugin)
///         .run();
/// }
/// ```
///
/// If you want to setup your own `Ctrl+C` handler, you should call the
/// [`TerminalCtrlCHandlerPlugin::gracefully_exit`] function in your handler if you want bevy to gracefully exit.
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, TerminalCtrlCHandlerPlugin, ctrlc};
/// fn main() {
///     // Your own `Ctrl+C` handler
///     ctrlc::set_handler(move || {
///         // Other clean up code ...
///
///         TerminalCtrlCHandlerPlugin::gracefully_exit();
///     });
///
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .run();
/// }
/// ```
#[derive(Default)]
pub struct TerminalCtrlCHandlerPlugin;

impl TerminalCtrlCHandlerPlugin {
    /// Sends the [`AppExit`] event to all apps using this plugin to make them gracefully exit.
    pub fn gracefully_exit() {
        SHOULD_EXIT.store(true, Ordering::Relaxed);
    }

    /// Sends a [`AppExit`] event when the user presses `Ctrl+C` on the terminal.
    pub fn exit_on_flag(mut events: EventWriter<AppExit>) {
        if SHOULD_EXIT.load(Ordering::Relaxed) {
            events.write(AppExit::from_code(130));
        }
    }
}

impl Plugin for TerminalCtrlCHandlerPlugin {
    fn build(&self, app: &mut App) {
        let result = ctrlc::try_set_handler(move || {
            Self::gracefully_exit();
        });
        match result {
            Ok(()) => {}
            Err(ctrlc::Error::MultipleHandlers) => {
                log::info!("Skipping installing `Ctrl+C` handler as one was already installed. Please call `TerminalCtrlCHandlerPlugin::gracefully_exit` in your own `Ctrl+C` handler if you want Bevy to gracefully exit on `Ctrl+C`.");
            }
            Err(err) => log::warn!("Failed to set `Ctrl+C` handler: {err}"),
        }

        app.add_systems(Update, TerminalCtrlCHandlerPlugin::exit_on_flag);
    }
}
