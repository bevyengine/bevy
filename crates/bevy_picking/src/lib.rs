//! This crate provides 'picking' capabilities for the Bevy game engine, allowing pointers to
//! interact with entities using hover, click, and drag events.
//!
//! ## Overview
//!
//! In the simplest case, this plugin allows you to click on things in the scene. However, it also
//! allows you to express more complex interactions, like detecting when a touch input drags a UI
//! element and drops it on a 3d mesh rendered to a different camera.
//!
//! Pointer events bubble up the entity hierarchy and can be used with observers, allowing you to
//! succinctly express rich interaction behaviors by attaching pointer callbacks to entities:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_picking::prelude::*;
//! # #[derive(Component)]
//! # struct MyComponent;
//! # let mut world = World::new();
//! world.spawn(MyComponent)
//!     .observe(|mut event: On<Pointer<Click>>| {
//!         // Read the underlying pointer event data
//!         println!("Pointer {:?} was just clicked!", event.pointer_id);
//!         // Stop the event from bubbling up the entity hierarchy
//!         event.propagate(false);
//!     });
//! ```
//!
//! At its core, this crate provides a robust abstraction for computing picking state regardless of
//! pointing devices, or what you are hit testing against. It is designed to work with any input,
//! including mouse, touch, pens, or virtual pointers controlled by gamepads.
//!
//! ## Expressive Events
//!
//! Although the events in this module (see [`events`]) can be listened to with normal
//! `EventReader`s, using observers is often more expressive, with less boilerplate. This is because
//! observers allow you to attach event handling logic to specific entities, as well as make use of
//! event bubbling.
//!
//! When events are generated, they bubble up the entity hierarchy starting from their target, until
//! they reach the root or bubbling is halted with a call to
//! [`On::propagate`](bevy_ecs::observer::On::propagate). See [`Observer`] for details.
//!
//! This allows you to run callbacks when any children of an entity are interacted with, and leads
//! to succinct, expressive code:
//!
//! ```
//! # use bevy_ecs::prelude::*;
//! # use bevy_transform::prelude::*;
//! # use bevy_picking::prelude::*;
//! # #[derive(BufferedEvent)]
//! # struct Greeting;
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Transform::default())
//!         // Spawn your entity here, e.g. a `Mesh3d`.
//!         // When dragged, mutate the `Transform` component on the dragged target entity:
//!         .observe(|event: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>| {
//!             let mut transform = transforms.get_mut(event.entity()).unwrap();
//!             transform.rotate_local_y(event.delta.x / 50.0);
//!         })
//!         .observe(|event: On<Pointer<Click>>, mut commands: Commands| {
//!             println!("Entity {} goes BOOM!", event.entity());
//!             commands.entity(event.entity()).despawn();
//!         })
//!         .observe(|event: On<Pointer<Over>>, mut events: EventWriter<Greeting>| {
//!             events.write(Greeting);
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
//! Picking provides a generic Pointer abstraction, which is useful for reacting to many different
//! types of input devices. Pointers can be controlled with anything, whether it's the included
//! mouse or touch inputs, or a custom gamepad input system you write yourself to control a virtual
//! pointer.
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
//! [examples](https://github.com/bevyengine/bevy/tree/main/examples/picking). You can read the next
//! section to understand how the plugin works.
//!
//! # The Picking Pipeline
//!
//! This plugin is designed to be extremely modular. To do so, it works in well-defined stages that
//! form a pipeline, where events are used to pass data between each stage.
//!
//! #### Pointers ([`pointer`](mod@pointer))
//!
//! The first stage of the pipeline is to gather inputs and update pointers. This stage is
//! ultimately responsible for generating [`PointerInput`](pointer::PointerInput) events. The
//! provided crate does this automatically for mouse, touch, and pen inputs. If you wanted to
//! implement your own pointer, controlled by some other input, you can do that here. The ordering
//! of events within the [`PointerInput`](pointer::PointerInput) stream is meaningful for events
//! with the same [`PointerId`](pointer::PointerId), but not between different pointers.
//!
//! Because pointer positions and presses are driven by these events, you can use them to mock
//! inputs for testing.
//!
//! After inputs are generated, they are then collected to update the current
//! [`PointerLocation`](pointer::PointerLocation) for each pointer.
//!
//! #### Backend ([`backend`])
//!
//! A picking backend only has one job: reading [`PointerLocation`](pointer::PointerLocation)
//! components, and producing [`PointerHits`](backend::PointerHits). You can find all documentation
//! and types needed to implement a backend at [`backend`].
//!
//! You will eventually need to choose which picking backend(s) you want to use. This crate does not
//! supply any backends, and expects you to select some from the other bevy crates or the
//! third-party ecosystem.
//!
//! It's important to understand that you can mix and match backends! For example, you might have a
//! backend for your UI, and one for the 3d scene, with each being specialized for their purpose.
//! Bevy provides some backends out of the box, but you can even write your own. It's been made as
//! easy as possible intentionally; the `bevy_mod_raycast` backend is 50 lines of code.
//!
//! #### Hover ([`hover`])
//!
//! The next step is to use the data from the backends, combine and sort the results, and determine
//! what each cursor is hovering over, producing a [`HoverMap`](`crate::hover::HoverMap`). Note that
//! just because a pointer is over an entity, it is not necessarily *hovering* that entity. Although
//! multiple backends may be reporting that a pointer is hitting an entity, the hover system needs
//! to determine which entities are actually being hovered by this pointer based on the pick depth,
//! order of the backend, and the optional [`Pickable`] component of the entity. In other
//! words, if one entity is in front of another, usually only the topmost one will be hovered.
//!
//! #### Events ([`events`])
//!
//! In the final step, the high-level pointer events are generated, such as events that trigger when
//! a pointer hovers or clicks an entity. These simple events are then used to generate more complex
//! events for dragging and dropping.
//!
//! Because it is completely agnostic to the earlier stages of the pipeline, you can easily extend
//! the plugin with arbitrary backends and input methods, yet still use all the high level features.

#![deny(missing_docs)]

extern crate alloc;

pub mod backend;
pub mod events;
pub mod hover;
pub mod input;
#[cfg(feature = "bevy_mesh_picking_backend")]
pub mod mesh_picking;
pub mod pointer;
pub mod window;

use bevy_app::{prelude::*, PluginGroupBuilder};
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use hover::{update_is_directly_hovered, update_is_hovered};

/// The picking prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_mesh_picking_backend")]
    #[doc(hidden)]
    pub use crate::mesh_picking::{
        ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastBackfaces, RayCastVisibility},
        MeshPickingCamera, MeshPickingPlugin, MeshPickingSettings,
    };
    #[doc(hidden)]
    pub use crate::{
        events::*, input::PointerInputPlugin, pointer::PointerButton, DefaultPickingPlugins,
        InteractionPlugin, Pickable, PickingPlugin,
    };
}

/// An optional component that marks an entity as usable by a backend, and overrides default
/// picking behavior for an entity.
///
/// This allows you to make an entity non-hoverable, or allow items below it to be hovered.
///
/// See the documentation on the fields for more details.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
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
    /// so if the UI is drawing on top of the 3d mesh, the UI will be "above" the mesh. When hovering
    /// is computed, the UI element will be checked first to see if it this field is set to block
    /// lower entities. If it does (default), the hovering system will stop there, and only the UI
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

/// Groups the stages of the picking process under shared labels.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum PickingSystems {
    /// Produces pointer input events. In the [`First`] schedule.
    Input,
    /// Runs after input events are generated but before commands are flushed. In the [`First`]
    /// schedule.
    PostInput,
    /// Receives and processes pointer input events. In the [`PreUpdate`] schedule.
    ProcessInput,
    /// Reads inputs and produces [`backend::PointerHits`]s. In the [`PreUpdate`] schedule.
    Backend,
    /// Reads [`backend::PointerHits`]s, and updates the hovermap, selection, and highlighting states. In
    /// the [`PreUpdate`] schedule.
    Hover,
    /// Runs after all the [`PickingSystems::Hover`] systems are done, before event listeners are triggered. In the
    /// [`PreUpdate`] schedule.
    PostHover,
    /// Runs after all other picking sets. In the [`PreUpdate`] schedule.
    Last,
}

/// Deprecated alias for [`PickingSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `PickingSystems`.")]
pub type PickSet = PickingSystems;

/// One plugin that contains the [`PointerInputPlugin`](input::PointerInputPlugin), [`PickingPlugin`]
/// and the [`InteractionPlugin`], this is probably the plugin that will be most used.
///
/// Note: for any of these plugins to work, they require a picking backend to be active,
/// The picking backend is responsible to turn an input, into a [`crate::backend::PointerHits`]
/// that [`PickingPlugin`] and [`InteractionPlugin`] will refine into [`bevy_ecs::observer::On`]s.
#[derive(Default)]
pub struct DefaultPickingPlugins;

impl PluginGroup for DefaultPickingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(input::PointerInputPlugin)
            .add(PickingPlugin)
            .add(InteractionPlugin)
    }
}

#[derive(Copy, Clone, Debug, Resource, Reflect)]
#[reflect(Resource, Default, Debug, Clone)]
/// Controls the behavior of picking
///
/// ## Custom initialization
/// ```
/// # use bevy_app::App;
/// # use bevy_picking::{PickingSettings, PickingPlugin};
/// App::new()
///     .insert_resource(PickingSettings {
///         is_enabled: true,
///         is_input_enabled: false,
///         is_hover_enabled: true,
///         is_window_picking_enabled: false,
///     })
///     // or DefaultPlugins
///     .add_plugins(PickingPlugin);
/// ```
pub struct PickingSettings {
    /// Enables and disables all picking features.
    pub is_enabled: bool,
    /// Enables and disables input collection.
    pub is_input_enabled: bool,
    /// Enables and disables updating interaction states of entities.
    pub is_hover_enabled: bool,
    /// Enables or disables picking for window entities.
    pub is_window_picking_enabled: bool,
}

impl PickingSettings {
    /// Whether or not input collection systems should be running.
    pub fn input_should_run(state: Res<Self>) -> bool {
        state.is_input_enabled && state.is_enabled
    }

    /// Whether or not systems updating entities' [`PickingInteraction`](hover::PickingInteraction)
    /// component should be running.
    pub fn hover_should_run(state: Res<Self>) -> bool {
        state.is_hover_enabled && state.is_enabled
    }

    /// Whether or not window entities should receive pick events.
    pub fn window_picking_should_run(state: Res<Self>) -> bool {
        state.is_window_picking_enabled && state.is_enabled
    }
}

impl Default for PickingSettings {
    fn default() -> Self {
        Self {
            is_enabled: true,
            is_input_enabled: true,
            is_hover_enabled: true,
            is_window_picking_enabled: true,
        }
    }
}

/// This plugin sets up the core picking infrastructure. It receives input events, and provides the shared
/// types used by other picking plugins.
///
/// Behavior of picking can be controlled by modifying [`PickingSettings`].
///
/// [`PickingSettings`] will be initialized with default values if it
/// is not present at the moment this is added to the app.
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PickingSettings>()
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
                    .in_set(PickingSystems::ProcessInput),
            )
            .add_systems(
                PreUpdate,
                window::update_window_hits
                    .run_if(PickingSettings::window_picking_should_run)
                    .in_set(PickingSystems::Backend),
            )
            .configure_sets(
                First,
                (PickingSystems::Input, PickingSystems::PostInput)
                    .after(bevy_time::TimeSystems)
                    .after(bevy_ecs::event::EventUpdateSystems)
                    .chain(),
            )
            .configure_sets(
                PreUpdate,
                (
                    PickingSystems::ProcessInput.run_if(PickingSettings::input_should_run),
                    PickingSystems::Backend,
                    PickingSystems::Hover.run_if(PickingSettings::hover_should_run),
                    PickingSystems::PostHover,
                    PickingSystems::Last,
                )
                    .chain(),
            );
    }
}

/// Generates [`Pointer`](events::Pointer) events and handles event bubbling.
#[derive(Default)]
pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        use events::*;
        use hover::{generate_hovermap, update_interactions};

        app.init_resource::<hover::HoverMap>()
            .init_resource::<hover::PreviousHoverMap>()
            .init_resource::<PointerState>()
            .add_event::<Pointer<Cancel>>()
            .add_event::<Pointer<Click>>()
            .add_event::<Pointer<Press>>()
            .add_event::<Pointer<DragDrop>>()
            .add_event::<Pointer<DragEnd>>()
            .add_event::<Pointer<DragEnter>>()
            .add_event::<Pointer<Drag>>()
            .add_event::<Pointer<DragLeave>>()
            .add_event::<Pointer<DragOver>>()
            .add_event::<Pointer<DragStart>>()
            .add_event::<Pointer<Move>>()
            .add_event::<Pointer<Out>>()
            .add_event::<Pointer<Over>>()
            .add_event::<Pointer<Release>>()
            .add_event::<Pointer<Scroll>>()
            .add_systems(
                PreUpdate,
                (
                    generate_hovermap,
                    update_interactions,
                    (update_is_hovered, update_is_directly_hovered),
                    pointer_events,
                )
                    .chain()
                    .in_set(PickingSystems::Hover),
            );
    }
}
