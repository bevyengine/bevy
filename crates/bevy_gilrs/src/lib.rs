#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Systems and type definitions for gamepad handling in Bevy.
//!
//! This crate is built on top of [GilRs](gilrs), a library
//! that handles abstracting over platform-specific gamepad APIs.

mod converter;
mod gilrs_system;
mod rumble;

#[cfg(not(target_arch = "wasm32"))]
use bevy_platform::cell::SyncCell;

#[cfg(target_arch = "wasm32")]
use core::cell::RefCell;

use bevy_app::{App, Plugin, PostUpdate, PreStartup, PreUpdate};
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::prelude::*;
use bevy_input::InputSystems;
use bevy_platform::collections::HashMap;
use gilrs::GilrsBuilder;
use gilrs_system::{gilrs_event_startup_system, gilrs_event_system};
use rumble::{play_gilrs_rumble, RunningRumbleEffects};
use tracing::error;

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Temporary storage of gilrs data to replace usage of `!Send` resources. This will be replaced with proper
    /// storage of `!Send` data after issue #17667 is complete.
    ///
    /// Using a `thread_local!` here relies on the fact that wasm32 can only be single threaded. Previously, we used a
    /// `NonSendMut` parameter, which told Bevy that the system was `!Send`, but now with the removal of `!Send`
    /// resource/system parameter usage, there is no internal guarantee that the system will run in only one thread, so
    /// we need to rely on the platform to make such a guarantee.
    pub static GILRS: RefCell<Option<gilrs::Gilrs>> = const { RefCell::new(None) };
}

#[derive(Resource)]
pub(crate) struct Gilrs {
    #[cfg(not(target_arch = "wasm32"))]
    cell: SyncCell<gilrs::Gilrs>,
}

impl Gilrs {
    #[inline]
    pub fn with(&mut self, f: impl FnOnce(&mut gilrs::Gilrs)) {
        #[cfg(target_arch = "wasm32")]
        GILRS.with(|g| f(g.borrow_mut().as_mut().expect("GILRS was not initialized")));
        #[cfg(not(target_arch = "wasm32"))]
        f(self.cell.get());
    }
}

/// A [`resource`](Resource) with the mapping of connected [`gilrs::GamepadId`] and their [`Entity`].
#[derive(Debug, Default, Resource)]
pub(crate) struct GilrsGamepads {
    /// Mapping of [`Entity`] to [`gilrs::GamepadId`].
    pub(crate) entity_to_id: EntityHashMap<gilrs::GamepadId>,
    /// Mapping of [`gilrs::GamepadId`] to [`Entity`].
    pub(crate) id_to_entity: HashMap<gilrs::GamepadId, Entity>,
}

impl GilrsGamepads {
    /// Returns the [`Entity`] assigned to a connected [`gilrs::GamepadId`].
    pub fn get_entity(&self, gamepad_id: gilrs::GamepadId) -> Option<Entity> {
        self.id_to_entity.get(&gamepad_id).copied()
    }

    /// Returns the [`gilrs::GamepadId`] assigned to a gamepad [`Entity`].
    pub fn get_gamepad_id(&self, entity: Entity) -> Option<gilrs::GamepadId> {
        self.entity_to_id.get(&entity).copied()
    }
}

/// Plugin that provides gamepad handling to an [`App`].
#[derive(Default)]
pub struct GilrsPlugin;

/// Updates the running gamepad rumble effects.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub struct RumbleSystems;

/// Deprecated alias for [`RumbleSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `RumbleSystems`.")]
pub type RumbleSystem = RumbleSystems;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut App) {
        match GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build()
        {
            Ok(gilrs) => {
                let g = Gilrs {
                    #[cfg(not(target_arch = "wasm32"))]
                    cell: SyncCell::new(gilrs),
                };
                #[cfg(target_arch = "wasm32")]
                GILRS.with(|g| {
                    g.replace(Some(gilrs));
                });
                app.insert_resource(g);
                app.init_resource::<GilrsGamepads>();
                app.init_resource::<RunningRumbleEffects>()
                    .add_systems(PreStartup, gilrs_event_startup_system)
                    .add_systems(PreUpdate, gilrs_event_system.before(InputSystems))
                    .add_systems(PostUpdate, play_gilrs_rumble.in_set(RumbleSystems));
            }
            Err(err) => error!("Failed to start Gilrs. {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression test for https://github.com/bevyengine/bevy/issues/17697
    #[test]
    fn world_is_truly_send() {
        let mut app = App::new();
        app.add_plugins(GilrsPlugin);
        let world = core::mem::take(app.world_mut());

        let handler = std::thread::spawn(move || {
            drop(world);
        });

        handler.join().unwrap();
    }
}
