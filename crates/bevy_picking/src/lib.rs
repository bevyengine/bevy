//! This crate provides 'picking' capabilities for the Bevy game engine. That means, in simple terms, figuring out
//! how to connect up a user's clicks or taps to the entities they are trying to interact with.
//!
//! ## Overview
//!
//! In the simplest case, this plugin allows you to click on things in the scene. However, it also
//! allows you to express more complex interactions, like detecting when a touch input drags a UI
//! element and drops it on a 3d mesh rendered to a different camera. The crate also provides a set of
//! interaction callbacks, allowing you to receive input directly on entities like here:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_picking::prelude::*;
//! # #[derive(Component)]
//! # struct MyComponent;
//! # let mut world = World::new();
//! world.spawn(MyComponent)
//!     .observe(|mut trigger: Trigger<Pointer<Click>>| {
//!         // Get the underlying event type
//!         let click_event: &Pointer<Click> = trigger.event();
//!         // Stop the event from bubbling up the entity hierarchjy
//!         trigger.propagate(false);
//!     });
//! ```
//!
//! At its core, this crate provides a robust abstraction for computing picking state regardless of
//! pointing devices, or what you are hit testing against. It is designed to work with any input, including
//! mouse, touch, pens, or virtual pointers controlled by gamepads.
//!
//! ## Expressive Events
//!
//! The events in this module (see [`events`]) cannot be listened to with normal `EventReader`s.
//! Instead, they are dispatched to *ovservers* attached to specific entities. When events are generated, they
//! bubble up the entity hierarchy starting from their target, until they reach the root or bubbling is haulted
//! with a call to [`Trigger::propagate`](bevy_ecs::observer::Trigger::propagate).
//! See [`Observer`] for details.
//!
//! This allows you to run callbacks when any children of an entity are interacted with, and leads
//! to succinct, expressive code:
//!
//! ```
//! # use bevy_ecs::prelude::*;
//! # use bevy_transform::prelude::*;
//! # use bevy_picking::prelude::*;
//! # #[derive(Event)]
//! # struct Greeting;
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Transform::default())
//!         // Spawn your entity here, e.g. a Mesh.
//!         // When dragged, mutate the `Transform` component on the dragged target entity:
//!         .observe(|trigger: Trigger<Pointer<Drag>>, mut transforms: Query<&mut Transform>| {
//!             let mut transform = transforms.get_mut(trigger.entity()).unwrap();
//!             let drag = trigger.event();
//!             transform.rotate_local_y(drag.delta.x / 50.0);
//!         })
//!         .observe(|trigger: Trigger<Pointer<Click>>, mut commands: Commands| {
//!             println!("Entity {:?} goes BOOM!", trigger.entity());
//!             commands.entity(trigger.entity()).despawn();
//!         })
//!         .observe(|trigger: Trigger<Pointer<Over>>, mut events: EventWriter<Greeting>| {
//!             events.send(Greeting);
//!         });
//! }
//! ```
//!
//! ## Modularity
//!
//! #### Mix and Match Hit Testing Backends
//!
//! The plugin attempts to handle all the hard parts for you, all you need to do is tell it when a
//! pointer is hitting any entities. Multiple backends can be used at the same time! [Use this
//! simple API to write your own backend](crate::backend) in about 100 lines of code.
//!
//! #### Input Agnostic
//!
//! Picking provides a generic Pointer abstracton, which is useful for reacting to many different
//! types of input devices. Pointers can be controlled with anything, whether its the included mouse
//! or touch inputs, or a custom gamepad input system you write yourself to control a virtual pointer.
//!
//! ## Robustness
//!
//! In addition to these features, this plugin also correctly handles multitouch, multiple windows,
//! multiple cameras, viewports, and render layers. Using this as a library allows you to write a
//! picking backend that can interoperate with any other picking backend.
//!
//! # Getting Started
//!
//! TODO: This section will need to be re-written once more backends are introduced.
//!
//! #### Next Steps
//!
//! To learn more, take a look at the examples in the
//! [examples](https://github.com/bevyengine/bevy/tree/main/examples/picking). You
//! can read the next section to understand how the plugin works.
//!
//! # The Picking Pipeline
//!
//! This plugin is designed to be extremely modular. To do so, it works in well-defined stages that
//! form a pipeline, where events are used to pass data between each stage.
//!
//! #### Pointers ([`pointer`](mod@pointer))
//!
//! The first stage of the pipeline is to gather inputs and update pointers. This stage is
//! ultimately responsible for generating [`PointerInput`](pointer::PointerInput) events. The provided
//! crate does this automatically for mouse, touch, and pen inputs. If you wanted to implement your own
//! pointer, controlled by some other input, you can do that here. The ordering of events within the
//! [`PointerInput`](pointer::PointerInput) stream is meaningful for events with the same
//! [`PointerId`](pointer::PointerId), but not between different pointers.
//!
//! Because pointer positions and presses are driven by these events, you can use them to mock
//! inputs for testing.
//!
//! After inputs are generated, they are then collected to update the current
//! [`PointerLocation`](pointer::PointerLocation) for each pointer.
//!
//! #### Backend ([`backend`])
//!
//! A picking backend only has one job: reading [`PointerLocation`](pointer::PointerLocation) components,
//! and producing [`PointerHits`](backend::PointerHits). You can find all documentation and types needed to
//! implement a backend at [`backend`].
//!
//! You will eventually need to choose which picking backend(s) you want to use. This crate does not
//! supply any backends, and expects you to select some from the other bevy crates or the third-party
//! ecosystem. You can find all the provided backends in the [`backend`] module.
//!
//! It's important to understand that you can mix and match backends! For example, you might have a
//! backend for your UI, and one for the 3d scene, with each being specialized for their purpose.
//! This crate provides some backends out of the box, but you can even write your own. It's been
//! made as easy as possible intentionally; the `bevy_mod_raycast` backend is 50 lines of code.
//!
//! #### Focus ([`focus`])
//!
//! The next step is to use the data from the backends, combine and sort the results, and determine
//! what each cursor is hovering over, producing a [`HoverMap`](`crate::focus::HoverMap`). Note that
//! just because a pointer is over an entity, it is not necessarily *hovering* that entity. Although
//! multiple backends may be reporting that a pointer is hitting an entity, the focus system needs
//! to determine which entities are actually being hovered by this pointer based on the pick depth,
//! order of the backend, and the [`Pickable`] state of the entity. In other words, if one entity is
//! in front of another, usually only the topmost one will be hovered.
//!
//! #### Events ([`events`])
//!
//! In the final step, the high-level pointer events are generated, such as events that trigger when
//! a pointer hovers or clicks an entity. These simple events are then used to generate more complex
//! events for dragging and dropping.
//!
//! Because it is completely agnostic to the the earlier stages of the pipeline, you can easily
//! extend the plugin with arbitrary backends and input methods, yet still use all the high level
//! features.

#![deny(missing_docs)]

pub mod backend;
pub mod events;
pub mod focus;
pub mod input;
pub mod pointer;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// The picking prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        events::*, input::PointerInputPlugin, pointer::PointerButton, DefaultPickingPlugins,
        InteractionPlugin, Pickable, PickingPlugin,
    };
}

/// An optional component that overrides default picking behavior for an entity, allowing you to
/// make an entity non-hoverable, or allow items below it to be hovered. See the documentation on
/// the fields for more details.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Debug, PartialEq)]
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

/// One plugin that contains the [`PointerInputPlugin`](input::PointerInputPlugin), [`PickingPlugin`]
/// and the [`InteractionPlugin`], this is probably the plugin that will be most used.
///
/// Note: for any of these plugins to work, they require a picking backend to be active,
/// The picking backend is responsible to turn an input, into a [`crate::backend::PointerHits`]
/// that [`PickingPlugin`] and [`InteractionPlugin`] will refine into [`bevy_ecs::observer::Trigger`]s.
#[derive(Default)]
pub struct DefaultPickingPlugins;

impl Plugin for DefaultPickingPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::PointerInputPlugin::default(),
            PickingPlugin::default(),
            InteractionPlugin,
        ));
    }
}

/// This plugin sets up the core picking infrastructure. It receives input events, and provides the shared
/// types used by other picking plugins.
///
/// This plugin contains several settings, and is added to the wrold as a resource after initialization. You
/// can configure picking settings at runtime through the resource.
#[derive(Copy, Clone, Debug, Resource, Reflect)]
#[reflect(Resource, Default, Debug)]
pub struct PickingPlugin {
    /// Enables and disables all picking features.
    pub is_enabled: bool,
    /// Enables and disables input collection.
    pub is_input_enabled: bool,
    /// Enables and disables updating interaction states of entities.
    pub is_focus_enabled: bool,
}

impl PickingPlugin {
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

impl Default for PickingPlugin {
    fn default() -> Self {
        Self {
            is_enabled: true,
            is_input_enabled: true,
            is_focus_enabled: true,
        }
    }
}

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(*self)
            .init_resource::<pointer::PointerMap>()
            .init_resource::<backend::ray::RayMap>()
            .add_event::<pointer::PointerInput>()
            .add_event::<backend::PointerHits>()
            // Rather than try to mark all current and future backends as ambiguous with each other,
            // we allow them to send their hits in any order. These are later sorted, so submission
            // order doesn't matter. See `PointerHits` docs for caveats.
            .allow_ambiguous_resource::<Events<backend::PointerHits>>()
            .add_systems(
                PreUpdate,
                (
                    pointer::update_pointer_map,
                    pointer::PointerInput::receive,
                    backend::ray::RayMap::repopulate.after(pointer::PointerInput::receive),
                )
                    .in_set(PickSet::ProcessInput),
            )
            .configure_sets(
                First,
                (PickSet::Input, PickSet::PostInput)
                    .after(bevy_time::TimeSystem)
                    .after(bevy_ecs::event::EventUpdates)
                    .chain(),
            )
            .configure_sets(
                PreUpdate,
                (
                    PickSet::ProcessInput.run_if(Self::input_should_run),
                    PickSet::Backend,
                    PickSet::Focus.run_if(Self::focus_should_run),
                    PickSet::PostFocus,
                    PickSet::Last,
                )
                    .chain(),
            )
            .register_type::<Self>()
            .register_type::<Pickable>()
            .register_type::<pointer::PointerId>()
            .register_type::<pointer::PointerLocation>()
            .register_type::<pointer::PointerPress>()
            .register_type::<pointer::PointerInteraction>()
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
            .add_systems(
                PreUpdate,
                (update_focus, pointer_events, update_interactions)
                    .chain()
                    .in_set(PickSet::Focus),
            );
    }
}
