//! `bevy_picking::input` is a thin layer that provides unsurprising default inputs to `bevy_picking`.
//! The included systems are responsible for sending  mouse and touch inputs to their
//! respective `Pointer`s.
//!
//! Because this resides in its own crate, it's easy to omit it, and provide your own inputs as
//! needed. Because `Pointer`s aren't coupled to the underlying input hardware, you can easily mock
//! inputs, and allow users full accessibility to map whatever inputs they need to pointer input.
//!
//! If, for example, you wanted to add support for VR input, all you need to do is spawn a pointer
//! entity with a custom [`PointerId`](crate::pointer::PointerId), and write a system
//! that updates its position.
//!
//! TODO: Update docs

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

use crate::PickSet;

pub mod mouse;
pub mod touch;

/// Common imports for `bevy_picking_input`.
pub mod prelude {
    pub use crate::input::InputPlugin;
}

/// Adds mouse and touch inputs for picking pointers to your app. This is a default input plugin,
/// that you can replace with your own plugin as needed.
///
/// [`crate::PickingPluginsSettings::is_input_enabled`] can be used to toggle whether
/// the core picking plugin processes the inputs sent by this, or other input plugins, in one place.
#[derive(Copy, Clone, Resource, Debug, Reflect)]
#[reflect(Resource, Default)]
pub struct InputPlugin {
    /// Should touch inputs be updated?
    pub is_touch_enabled: bool,
    /// Should mouse inputs be updated?
    pub is_mouse_enabled: bool,
}

impl InputPlugin {
    fn is_mouse_enabled(state: Res<Self>) -> bool {
        state.is_mouse_enabled
    }

    fn is_touch_enabled(state: Res<Self>) -> bool {
        state.is_touch_enabled
    }
}

impl Default for InputPlugin {
    fn default() -> Self {
        Self {
            is_touch_enabled: true,
            is_mouse_enabled: true,
        }
    }
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(*self)
            .add_systems(Startup, mouse::spawn_mouse_pointer)
            .add_systems(
                First,
                (
                    mouse::mouse_pick_events.run_if(InputPlugin::is_mouse_enabled),
                    touch::touch_pick_events.run_if(InputPlugin::is_touch_enabled),
                    // IMPORTANT: the commands must be flushed after `touch_pick_events` is run
                    // because we need pointer spawning to happen immediately to prevent issues with
                    // missed events during drag and drop.
                    apply_deferred,
                )
                    .chain()
                    .in_set(PickSet::Input),
            )
            .add_systems(
                Last,
                touch::deactivate_touch_pointers.run_if(InputPlugin::is_touch_enabled),
            )
            .register_type::<Self>()
            .register_type::<InputPlugin>();
    }
}
