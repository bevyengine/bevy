//! TODO, write module doc

#![deny(missing_docs)]

pub mod backend;
pub mod events;
pub mod focus;
pub mod input;
pub mod pointer;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// common exports for picking interaction
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        events::*, input::InputPlugin, pointer::PointerButton, DefaultPickingPlugins,
        InteractionPlugin, Pickable, PickingPlugin, PickingPluginsSettings,
    };
}

/// Used to globally toggle picking features at runtime.
#[derive(Clone, Debug, Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct PickingPluginsSettings {
    /// Enables and disables all picking features.
    pub is_enabled: bool,
    /// Enables and disables input collection.
    pub is_input_enabled: bool,
    /// Enables and disables updating interaction states of entities.
    pub is_focus_enabled: bool,
}

impl PickingPluginsSettings {
    /// Whether or not input collection systems should be running.
    pub fn input_should_run(state: Res<Self>) -> bool {
        state.is_input_enabled && state.is_enabled
    }
    /// Whether or not systems updating entities' [`PickingInteraction`](focus::PickingInteraction)
    /// component should be running.
    pub fn focus_should_run(state: Res<Self>) -> bool {
        state.is_focus_enabled && state.is_enabled
    }
}

impl Default for PickingPluginsSettings {
    fn default() -> Self {
        Self {
            is_enabled: true,
            is_input_enabled: true,
            is_focus_enabled: true,
        }
    }
}

/// An optional component that overrides default picking behavior for an entity, allowing you to
/// make an entity non-hoverable, or allow items below it to be hovered. See the documentation on
/// the fields for more details.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct Pickable {
    /// Should this entity block entities below it from being picked?
    ///
    /// This is useful if you want picking to continue hitting entities below this one. Normally,
    /// only the topmost entity under a pointer can be hovered, but this setting allows the pointer
    /// to hover multiple entities, from nearest to farthest, stopping as soon as it hits an entity
    /// that blocks lower entities.
    ///
    /// Note that the word "lower" here refers to entities that have been reported as hit by any
    /// picking backend, but are at a lower depth than the current one. This is different from the
    /// concept of event bubbling, as it works irrespective of the entity hierarchy.
    ///
    /// For example, if a pointer is over a UI element, as well as a 3d mesh, backends will report
    /// hits for both of these entities. Additionally, the hits will be sorted by the camera order,
    /// so if the UI is drawing on top of the 3d mesh, the UI will be "above" the mesh. When focus
    /// is computed, the UI element will be checked first to see if it this field is set to block
    /// lower entities. If it does (default), the focus system will stop there, and only the UI
    /// element will be marked as hovered. However, if this field is set to `false`, both the UI
    /// element *and* the mesh will be marked as hovered.
    ///
    /// Entities without the [`Pickable`] component will block by default.
    pub should_block_lower: bool,

    /// If this is set to `false` and `should_block_lower` is set to true, this entity will block
    /// lower entities from being interacted and at the same time will itself not emit any events.
    ///
    /// Note that the word "lower" here refers to entities that have been reported as hit by any
    /// picking backend, but are at a lower depth than the current one. This is different from the
    /// concept of event bubbling, as it works irrespective of the entity hierarchy.
    ///
    /// For example, if a pointer is over a UI element, and this field is set to `false`, it will
    /// not be marked as hovered, and consequently will not emit events nor will any picking
    /// components mark it as hovered. This can be combined with the other field
    /// [`Self::should_block_lower`], which is orthogonal to this one.
    ///
    /// Entities without the [`Pickable`] component are hoverable by default.
    pub is_hoverable: bool,
}

impl Pickable {
    /// This entity will not block entities beneath it, nor will it emit events.
    ///
    /// If a backend reports this entity as being hit, the picking plugin will completely ignore it.
    pub const IGNORE: Self = Self {
        should_block_lower: false,
        is_hoverable: false,
    };
}

impl Default for Pickable {
    fn default() -> Self {
        Self {
            should_block_lower: true,
            is_hoverable: true,
        }
    }
}

/// Components needed to build a pointer. Multiple pointers can be active at once, with each pointer
/// being an entity.
///
/// `Mouse` and `Touch` pointers are automatically spawned as needed. Use this bundle if you are
/// spawning a custom `PointerId::Custom` pointer, either for testing, as a software controlled
/// pointer, or if you are replacing the default touch and mouse inputs.
#[derive(Bundle)]
pub struct PointerBundle {
    /// The pointer's unique [`PointerId`](pointer::PointerId).
    pub id: pointer::PointerId,
    /// Tracks the pointer's location.
    pub location: pointer::PointerLocation,
    /// Tracks the pointer's button press state.
    pub click: pointer::PointerPress,
    /// The interaction state of any hovered entities.
    pub interaction: pointer::PointerInteraction,
}

impl PointerBundle {
    /// Create a new pointer with the provided [`PointerId`](pointer::PointerId).
    pub fn new(id: pointer::PointerId) -> Self {
        PointerBundle {
            id,
            location: pointer::PointerLocation::default(),
            click: pointer::PointerPress::default(),
            interaction: pointer::PointerInteraction::default(),
        }
    }

    /// Sets the location of the pointer bundle
    pub fn with_location(mut self, location: pointer::Location) -> Self {
        self.location.location = Some(location);
        self
    }
}

/// Groups the stages of the picking process under shared labels.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum PickSet {
    /// Produces pointer input events. In the [`First`] schedule.
    Input,
    /// Runs after input events are generated but before commands are flushed. In the [`First`]
    /// schedule.
    PostInput,
    /// Receives and processes pointer input events. In the [`PreUpdate`] schedule.
    ProcessInput,
    /// Reads inputs and produces [`backend::PointerHits`]s. In the [`PreUpdate`] schedule.
    Backend,
    /// Reads [`backend::PointerHits`]s, and updates focus, selection, and highlighting states. In
    /// the [`PreUpdate`] schedule.
    Focus,
    /// Runs after all the focus systems are done, before event listeners are triggered. In the
    /// [`PreUpdate`] schedule.
    PostFocus,
    /// Runs after all other picking sets. In the [`PreUpdate`] schedule.
    Last,
}

/// One plugin that contains the [`input::InputPlugin`], [`PickingPlugin`] and the [`InteractionPlugin`],
/// this is probably the plugin that will be most used.
/// Note: for any of these plugins to work, they require a picking backend to be active,
/// The picking backend is responsible to turn an input, into a [`crate::backend::PointerHits`]
/// that [`PickingPlugin`] and [`InteractionPlugin`] will refine into [`bevy_ecs::observer::Trigger`]s.
#[derive(Default)]
pub struct DefaultPickingPlugins;

impl Plugin for DefaultPickingPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::InputPlugin::default(),
            PickingPlugin,
            InteractionPlugin,
        ));
    }
}

/// This plugin sets up the core picking infrastructure. It receives input events, and provides the shared
/// types used by other picking plugins.
#[derive(Default)]
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PickingPluginsSettings>()
            .init_resource::<pointer::PointerMap>()
            .init_resource::<backend::ray::RayMap>()
            .add_event::<pointer::InputPress>()
            .add_event::<pointer::InputMove>()
            .add_event::<backend::PointerHits>()
            // Rather than try to mark all current and future backends as ambiguous with each other,
            // we allow them to send their hits in any order. These are later sorted, so submission
            // order doesn't matter. See `PointerHits` docs for caveats.
            .allow_ambiguous_resource::<Events<backend::PointerHits>>()
            .add_systems(
                PreUpdate,
                (
                    pointer::update_pointer_map,
                    pointer::InputMove::receive,
                    pointer::InputPress::receive,
                    backend::ray::RayMap::repopulate.after(pointer::InputMove::receive),
                )
                    .in_set(PickSet::ProcessInput),
            )
            .configure_sets(
                First,
                (PickSet::Input, PickSet::PostInput)
                    .after(bevy_time::TimeSystem)
                    .ambiguous_with(bevy_asset::handle_internal_asset_events)
                    .after(bevy_ecs::event::EventUpdates)
                    .chain(),
            )
            .configure_sets(
                PreUpdate,
                (
                    PickSet::ProcessInput.run_if(PickingPluginsSettings::input_should_run),
                    PickSet::Backend,
                    PickSet::Focus.run_if(PickingPluginsSettings::focus_should_run),
                    PickSet::PostFocus,
                    // Eventually events will need to be dispatched here
                    PickSet::Last,
                )
                    .ambiguous_with(bevy_asset::handle_internal_asset_events)
                    .chain(),
            )
            .register_type::<pointer::PointerId>()
            .register_type::<pointer::PointerLocation>()
            .register_type::<pointer::PointerPress>()
            .register_type::<pointer::PointerInteraction>()
            .register_type::<Pickable>()
            .register_type::<PickingPluginsSettings>()
            .register_type::<backend::ray::RayId>();
    }
}

/// Generates [`Pointer`](events::Pointer) events and handles event bubbling.
#[derive(Default)]
pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        use events::*;
        use focus::{update_focus, update_interactions};

        app.init_resource::<focus::HoverMap>()
            .init_resource::<focus::PreviousHoverMap>()
            .init_resource::<DragMap>()
            .add_event::<PointerCancel>()
            .add_event::<Pointer<Down>>()
            .add_event::<Pointer<Up>>()
            .add_event::<Pointer<Move>>()
            .add_event::<Pointer<Over>>()
            .add_event::<Pointer<Out>>()
            .add_event::<Pointer<DragEnd>>()
            .add_systems(
                PreUpdate,
                (
                    update_focus,
                    pointer_events,
                    update_interactions,
                    send_click_and_drag_events,
                    send_drag_over_events,
                )
                    .chain()
                    .in_set(PickSet::Focus),
            );
    }
}
