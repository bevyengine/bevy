#[warn(missing_docs)]
mod cursor;
mod event;
mod raw_handle;
mod system;
mod window;
mod windows;

pub use crate::raw_handle::*;
pub use cursor::*;
pub use event::*;
pub use system::*;
pub use window::*;
pub use windows::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorIcon, CursorLeft, CursorMoved, FileDragAndDrop, MonitorSelection,
        ReceivedCharacter, Window, WindowDescriptor, WindowMode, WindowMoved, WindowPlugin,
        WindowPosition, Windows,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::schedule::{IntoSystemDescriptor, SystemLabel};

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            window: Default::default(),
            add_primary_window: true,
            exit_on_all_closed: true,
            close_when_requested: true,
        }
    }
}

/// A [`Plugin`] that defines an interface for windowing support in Bevy.
pub struct WindowPlugin {
    pub window: WindowDescriptor,
    /// Whether to create a window when added.
    ///
    /// Note that if there are no windows, by default the App will exit,
    /// due to [`exit_on_all_closed`].
    pub add_primary_window: bool,
    /// Whether to exit the app when there are no open windows.
    ///
    /// If disabling this, ensure that you send the [`bevy_app::AppExit`]
    /// event when the app should exit. If this does not occur, you will
    /// create 'headless' processes (processes without windows), which may
    /// surprise your users. It is recommended to leave this setting as `true`.
    ///
    /// If true, this plugin will add [`exit_on_all_closed`] to [`CoreStage::PostUpdate`].
    pub exit_on_all_closed: bool,
    /// Whether to close windows when they are requested to be closed (i.e.
    /// when the close button is pressed).
    ///
    /// If true, this plugin will add [`close_when_requested`] to [`CoreStage::Update`].
    /// If this system (or a replacement) is not running, the close button will have no effect.
    /// This may surprise your users. It is recommended to leave this setting as `true`.
    pub close_when_requested: bool,
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_event::<WindowClosed>()
            .add_event::<WindowCloseRequested>()
            .add_event::<RequestRedraw>()
            .add_event::<CursorMoved>()
            .add_event::<CursorEntered>()
            .add_event::<CursorLeft>()
            .add_event::<ReceivedCharacter>()
            .add_event::<WindowFocused>()
            .add_event::<WindowScaleFactorChanged>()
            .add_event::<WindowBackendScaleFactorChanged>()
            .add_event::<FileDragAndDrop>()
            .add_event::<WindowMoved>()
            .init_resource::<Windows>();

        if self.add_primary_window {
            app.world.send_event(CreateWindow {
                id: WindowId::primary(),
                descriptor: self.window.clone(),
            });
        }

        if self.exit_on_all_closed {
            app.add_system_to_stage(
                CoreStage::PostUpdate,
                exit_on_all_closed.after(ModifiesWindows),
            );
        }
        if self.close_when_requested {
            app.add_system(close_when_requested);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub struct ModifiesWindows;
