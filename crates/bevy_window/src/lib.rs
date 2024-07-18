#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! `bevy_window` provides a platform-agnostic interface for windowing in Bevy.
//!
//! This crate contains types for window management and events,
//! used by windowing implementors such as `bevy_winit`.
//! The [`WindowPlugin`] sets up some global window-related parameters and
//! is part of the [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html).

use std::sync::{Arc, Mutex};

use bevy_a11y::Focus;

mod event;
mod raw_handle;
mod system;
mod system_cursor;
mod window;

pub use crate::raw_handle::*;

pub use event::*;
pub use system::*;
pub use system_cursor::*;
pub use window::*;

#[allow(missing_docs)]
pub mod prelude {
    #[allow(deprecated)]
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime, MonitorSelection,
        ReceivedCharacter, Window, WindowMoved, WindowPlugin, WindowPosition,
        WindowResizeConstraints,
    };
}

use bevy_app::prelude::*;

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            primary_window: Some(Window::default()),
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
        #[allow(deprecated)]
        app.add_event::<WindowResized>()
            .add_event::<WindowCreated>()
            .add_event::<WindowClosing>()
            .add_event::<WindowClosed>()
            .add_event::<WindowCloseRequested>()
            .add_event::<WindowDestroyed>()
            .add_event::<RequestRedraw>()
            .add_event::<CursorMoved>()
            .add_event::<CursorEntered>()
            .add_event::<CursorLeft>()
            .add_event::<ReceivedCharacter>()
            .add_event::<Ime>()
            .add_event::<WindowFocused>()
            .add_event::<WindowOccluded>()
            .add_event::<WindowScaleFactorChanged>()
            .add_event::<WindowBackendScaleFactorChanged>()
            .add_event::<FileDragAndDrop>()
            .add_event::<WindowMoved>()
            .add_event::<WindowThemeChanged>()
            .add_event::<AppLifecycle>();

        if let Some(primary_window) = &self.primary_window {
            let initial_focus = app
                .world_mut()
                .spawn(primary_window.clone())
                .insert((
                    PrimaryWindow,
                    RawHandleWrapperHolder(Arc::new(Mutex::new(None))),
                ))
                .id();
            if let Some(mut focus) = app.world_mut().get_resource_mut::<Focus>() {
                **focus = Some(initial_focus);
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

        // Register event types
        #[allow(deprecated)]
        app.register_type::<WindowResized>()
            .register_type::<RequestRedraw>()
            .register_type::<WindowCreated>()
            .register_type::<WindowCloseRequested>()
            .register_type::<WindowClosing>()
            .register_type::<WindowClosed>()
            .register_type::<CursorMoved>()
            .register_type::<CursorEntered>()
            .register_type::<CursorLeft>()
            .register_type::<ReceivedCharacter>()
            .register_type::<WindowFocused>()
            .register_type::<WindowOccluded>()
            .register_type::<WindowScaleFactorChanged>()
            .register_type::<WindowBackendScaleFactorChanged>()
            .register_type::<FileDragAndDrop>()
            .register_type::<WindowMoved>()
            .register_type::<WindowThemeChanged>()
            .register_type::<AppLifecycle>();

        // Register window descriptor and related types
        app.register_type::<Window>()
            .register_type::<PrimaryWindow>();
    }
}

/// Defines the specific conditions the application should exit on
#[derive(Clone)]
pub enum ExitCondition {
    /// Close application when the primary window is closed
    ///
    /// The plugin will add [`exit_on_primary_closed`] to [`Update`].
    OnPrimaryClosed,
    /// Close application when all windows are closed
    ///
    /// The plugin will add [`exit_on_all_closed`] to [`Update`].
    OnAllClosed,
    /// Keep application running headless even after closing all windows
    ///
    /// If selecting this, ensure that you send the [`bevy_app::AppExit`]
    /// event when the app should exit. If this does not occur, you will
    /// create 'headless' processes (processes without windows), which may
    /// surprise your users.
    DontExit,
}
