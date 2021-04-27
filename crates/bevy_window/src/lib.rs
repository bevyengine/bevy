mod event;
mod system;
mod window;
mod windows;

use bevy_ecs::system::IntoSystem;
use bevy_math::Vec2;
use bevy_utils::HashMap;
pub use event::*;
pub use system::*;
pub use window::*;
pub use windows::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorLeft, CursorMoved, CursorPosition, FileDragAndDrop, ReceivedCharacter,
        Window, WindowDescriptor, WindowMoved, Windows,
    };
}

use bevy_app::{prelude::*, Events};

pub struct WindowPlugin {
    pub add_primary_window: bool,
    pub exit_on_close: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            add_primary_window: true,
            exit_on_close: true,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct CursorPosition {
    pub positions: HashMap<WindowId, Vec2>,
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_event::<WindowCloseRequested>()
            .add_event::<CloseWindow>()
            .add_event::<CursorMoved>()
            .init_resource::<CursorPosition>()
            .add_system_to_stage(CoreStage::PreUpdate, cursor_movement_res_system.system())
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
            let world = app.world_mut();
            let window_descriptor = world
                .get_resource::<WindowDescriptor>()
                .map(|descriptor| (*descriptor).clone())
                .unwrap_or_else(WindowDescriptor::default);
            let mut create_window_event = world.get_resource_mut::<Events<CreateWindow>>().unwrap();
            create_window_event.send(CreateWindow {
                id: WindowId::primary(),
                descriptor: window_descriptor,
            });
        }

        if self.exit_on_close {
            app.add_system(exit_on_window_close_system.system());
        }
    }
}
