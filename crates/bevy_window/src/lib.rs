#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! `bevy_window` provides a platform-agnostic interface for windowing in Bevy.
//!
//! This crate contains types for window management and events,
//! used by windowing implementors such as `bevy_winit`.
//! The [`WindowPlugin`] sets up some global window-related parameters and
//! is part of the [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html).

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod cursor;
mod event;
mod monitor;
mod raw_handle;
mod system;
mod window;

pub use crate::raw_handle::*;

pub use cursor::*;
pub use event::*;
pub use monitor::*;
pub use system::*;
pub use window::*;

/// The windowing prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime, MonitorSelection,
        VideoModeSelection, Window, WindowMoved, WindowPlugin, WindowPosition,
        WindowResizeConstraints,
    };
}

use alloc::sync::Arc;
use bevy_app::prelude::*;
use bevy_platform::sync::Mutex;

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            primary_window: Some(Window::default()),
            primary_cursor_options: Some(CursorOptions::default()),
            exit_condition: ExitCondition::OnAllClosed,
            close_when_requested: true,
        }
    }
}

/// A [`Plugin`] that defines an interface for windowing support in Bevy.
pub struct WindowPlugin {
    /// Settings for the primary window.
    ///
    /// `Some(custom_window)` will spawn an entity with `custom_window` and [`PrimaryWindow`] as components.
    /// `None` will not spawn a primary window.
    ///
    /// Defaults to `Some(Window::default())`.
    ///
    /// Note that if there are no windows the App will exit (by default) due to
    /// [`exit_on_all_closed`].
    pub primary_window: Option<Window>,

    /// Settings for the cursor on the primary window.
    ///
    /// Defaults to `Some(CursorOptions::default())`.
    ///
    /// Has no effect if [`WindowPlugin::primary_window`] is `None`.
    pub primary_cursor_options: Option<CursorOptions>,

    /// Whether to exit the app when there are no open windows.
    ///
    /// If disabling this, ensure that you send the [`bevy_app::AppExit`]
    /// event when the app should exit. If this does not occur, you will
    /// create 'headless' processes (processes without windows), which may
    /// surprise your users. It is recommended to leave this setting to
    /// either [`ExitCondition::OnAllClosed`] or [`ExitCondition::OnPrimaryClosed`].
    ///
    /// [`ExitCondition::OnAllClosed`] will add [`exit_on_all_closed`] to [`Update`].
    /// [`ExitCondition::OnPrimaryClosed`] will add [`exit_on_primary_closed`] to [`Update`].
    pub exit_condition: ExitCondition,

    /// Whether to close windows when they are requested to be closed (i.e.
    /// when the close button is pressed).
    ///
    /// If true, this plugin will add [`close_when_requested`] to [`Update`].
    /// If this system (or a replacement) is not running, the close button will have no effect.
    /// This may surprise your users. It is recommended to leave this setting as `true`.
    pub close_when_requested: bool,
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        // User convenience events
        app.add_message::<WindowEvent>()
            .add_message::<WindowResized>()
            .add_message::<WindowCreated>()
            .add_message::<WindowClosing>()
            .add_message::<WindowClosed>()
            .add_message::<WindowCloseRequested>()
            .add_message::<WindowDestroyed>()
            .add_message::<RequestRedraw>()
            .add_message::<CursorMoved>()
            .add_message::<CursorEntered>()
            .add_message::<CursorLeft>()
            .add_message::<Ime>()
            .add_message::<WindowFocused>()
            .add_message::<WindowOccluded>()
            .add_message::<WindowScaleFactorChanged>()
            .add_message::<WindowBackendScaleFactorChanged>()
            .add_message::<FileDragAndDrop>()
            .add_message::<WindowMoved>()
            .add_message::<WindowThemeChanged>()
            .add_message::<AppLifecycle>();

        if let Some(primary_window) = &self.primary_window {
            let mut entity_commands = app.world_mut().spawn(primary_window.clone());
            entity_commands.insert((
                PrimaryWindow,
                RawHandleWrapperHolder(Arc::new(Mutex::new(None))),
            ));
            if let Some(primary_cursor_options) = &self.primary_cursor_options {
                entity_commands.insert(primary_cursor_options.clone());
            }
        }

        match self.exit_condition {
            ExitCondition::OnPrimaryClosed => {
                app.add_systems(PostUpdate, exit_on_primary_closed);
            }
            ExitCondition::OnAllClosed => {
                app.add_systems(PostUpdate, exit_on_all_closed);
            }
            ExitCondition::DontExit => {}
        }

        if self.close_when_requested {
            // Need to run before `exit_on_*` systems
            app.add_systems(Update, close_when_requested);
        }
    }
}

/// Defines the specific conditions the application should exit on
#[derive(Clone)]
pub enum ExitCondition {
    /// Close application when the primary window is closed
    ///
    /// The plugin will add [`exit_on_primary_closed`] to [`PostUpdate`].
    OnPrimaryClosed,
    /// Close application when all windows are closed
    ///
    /// The plugin will add [`exit_on_all_closed`] to [`PostUpdate`].
    OnAllClosed,
    /// Keep application running headless even after closing all windows
    ///
    /// If selecting this, ensure that you send the [`bevy_app::AppExit`]
    /// event when the app should exit. If this does not occur, you will
    /// create 'headless' processes (processes without windows), which may
    /// surprise your users.
    DontExit,
}
