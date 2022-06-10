mod cursor;
mod event;
mod raw_window_handle;
mod system;
mod window;
mod window_commands;

pub use crate::raw_window_handle::*;
pub use cursor::*;
pub use event::*;
pub use system::*;
pub use window::*;
pub use window_commands::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorIcon, CursorLeft, CursorMoved, FileDragAndDrop, ReceivedCharacter,
        Window, WindowCommands, WindowCommandsExtension, WindowDescriptor, WindowMoved,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::{entity::Entity, event::Events, schedule::SystemLabel};

pub struct WindowPlugin {
    /// Whether to create a window when added.
    ///
    /// Note that if there are no windows, by default the App will exit,
    /// due to [`exit_on_all_closed`].
    pub add_primary_window: bool,
    /// Whether to exit the app when there are no open windows.
    /// If disabling this, ensure that you send the [`bevy_app::AppExit`]
    /// event when the app should exit. If this does not occur, you will
    /// create 'headless' processes (processes without windows), which may
    /// surprise your users. It is recommended to leave this setting as `true`.
    ///
    /// If true, this plugin will add [`exit_on_all_closed`] to [`CoreStage::Update`].
    // TODO: Update documentation here
    pub exit_condition: ExitCondition,
    /// Whether to close windows when they are requested to be closed (i.e.
    /// when the close button is pressed)
    ///
    /// If true, this plugin will add [`close_when_requested`] to [`CoreStage::Update`].
    /// If this system (or a replacement) is not running, the close button will have no effect.
    /// This may surprise your users. It is recommended to leave this setting as `true`.
    pub close_when_requested: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            add_primary_window: true,
            exit_condition: ExitCondition::OnAllClosed,
            close_when_requested: true,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowResized>()
            // TODO: This is now moved to a command and no longer needed
            // .add_event::<CreateWindow>()
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
            // Command events
            .add_event::<CreateWindowCommand>()
            .add_event::<SetWindowModeCommand>()
            .add_event::<SetTitleCommand>()
            .add_event::<SetScaleFactorCommand>()
            .add_event::<SetResolutionCommand>()
            .add_event::<SetPresentModeCommand>()
            .add_event::<SetResizableCommand>()
            .add_event::<SetDecorationsCommand>()
            .add_event::<SetCursorLockModeCommand>()
            .add_event::<SetCursorIconCommand>()
            .add_event::<SetCursorVisibilityCommand>()
            .add_event::<SetCursorPositionCommand>()
            .add_event::<SetMaximizedCommand>()
            .add_event::<SetMinimizedCommand>()
            .add_event::<SetPositionCommand>()
            .add_event::<SetResizeConstraintsCommand>()
            .add_event::<CloseWindowCommand>()
            .insert_resource(PrimaryWindow::default());

        if self.add_primary_window {
            // TODO: Creating window should be done through commands as entities instead of old way
            app.add_startup_system(create_primary_window);

            // let window_descriptor = app
            //     .world
            //     .get_resource::<WindowDescriptor>()
            //     .map(|descriptor| (*descriptor).clone())
            //     .unwrap_or_default();
            // let mut create_window_event = app.world.resource_mut::<Events<CreateWindow>>();

            // // TODO: Replace with commands
            // create_window_event.send(CreateWindow {
            //     entity: WindowId::primary(),
            //     descriptor: window_descriptor,
            // });
        }

        match self.exit_condition {
            ExitCondition::OnPrimaryClosed => {
                app.add_system(exit_on_primary_closed);
            }
            ExitCondition::OnAllClosed => {
                app.add_system(exit_on_all_closed);
            }
            ExitCondition::DontExit => {}
        }

        if self.close_when_requested {
            app.add_system(close_when_requested);
        }
    }
}

/// System Label marking when changes are applied to windows
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub struct ModifiesWindows;

pub enum ExitCondition {
    /// Close application when the primary window is closed
    OnPrimaryClosed,
    /// Close application when all windows are closed
    OnAllClosed,
    /// Keep application running headless even after closing all windows
    DontExit,
}

/// Resource containing the Entity that is currently considered the primary window
pub struct PrimaryWindow {
    // TODO:
    // Should this be Option?
    // should this be allowed to change?
    // If yes, what should be responsible for updating it?
    pub window: Option<Entity>,
}

impl Default for PrimaryWindow {
    fn default() -> Self {
        Self {
            window: Option::None,
        }
    }
}
