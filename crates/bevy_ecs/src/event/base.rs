use crate::change_detection::MaybeLocation;
use crate::component::ComponentId;
use crate::world::World;
use crate::{component::Component, traversal::Traversal};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// Something that "happens" and might be read / observed by app logic.
///
/// Events can be stored in an [`Events<E>`] resource
/// You can conveniently access events using the [`EventReader`] and [`EventWriter`] system parameter.
///
/// Events can also be "triggered" on a [`World`], which will then cause any [`Observer`] of that trigger to run.
///
/// Events must be thread-safe.
///
/// ## Derive
/// This trait can be derived.
/// Adding `auto_propagate` sets [`Self::AUTO_PROPAGATE`] to true.
/// Adding `traversal = "X"` sets [`Self::Traversal`] to be of type "X".
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(Event)]
/// #[event(auto_propagate)]
/// struct MyEvent;
/// ```
///
///
/// [`World`]: crate::world::World
/// [`ComponentId`]: crate::component::ComponentId
/// [`Observer`]: crate::observer::Observer
/// [`Events<E>`]: super::Events
/// [`EventReader`]: super::EventReader
/// [`EventWriter`]: super::EventWriter
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Event`",
    label = "invalid `Event`",
    note = "consider annotating `{Self}` with `#[derive(Event)]`"
)]
pub trait Event: Send + Sync + 'static {
    /// The component that describes which Entity to propagate this event to next, when [propagation] is enabled.
    ///
    /// [propagation]: crate::observer::Trigger::propagate
    type Traversal: Traversal<Self>;

    /// When true, this event will always attempt to propagate when [triggered], without requiring a call
    /// to [`Trigger::propagate`].
    ///
    /// [triggered]: crate::system::Commands::trigger_targets
    /// [`Trigger::propagate`]: crate::observer::Trigger::propagate
    const AUTO_PROPAGATE: bool = false;

    /// Generates the [`ComponentId`] for this event type.
    ///
    /// If this type has already been registered,
    /// this will return the existing [`ComponentId`].
    ///
    /// This is used by various dynamically typed observer APIs,
    /// such as [`World::trigger_targets_dynamic`].
    ///
    /// # Warning
    ///
    /// This method should not be overridden by implementors,
    /// and should always correspond to the implementation of [`component_id`](Event::component_id).
    fn register_component_id(world: &mut World) -> ComponentId {
        world.register_component::<EventWrapperComponent<Self>>()
    }

    /// Fetches the [`ComponentId`] for this event type,
    /// if it has already been generated.
    ///
    /// This is used by various dynamically typed observer APIs,
    /// such as [`World::trigger_targets_dynamic`].
    ///
    /// # Warning
    ///
    /// This method should not be overridden by implementors,
    /// and should always correspond to the implementation of [`register_component_id`](Event::register_component_id).
    fn component_id(world: &World) -> Option<ComponentId> {
        world.component_id::<EventWrapperComponent<Self>>()
    }
}

/// An internal type that implements [`Component`] for a given [`Event`] type.
///
/// This exists so we can easily get access to a unique [`ComponentId`] for each [`Event`] type,
/// without requiring that [`Event`] types implement [`Component`] directly.
/// [`ComponentId`] is used internally as a unique identitifier for events because they are:
///
/// - Unique to each event type.
/// - Can be quickly generated and looked up.
/// - Are compatible with dynamic event types, which aren't backed by a Rust type.
///
/// This type is an implementation detail and should never be made public.
// TODO: refactor events to store their metadata on distinct entities, rather than using `ComponentId`
#[derive(Component)]
struct EventWrapperComponent<E: Event + ?Sized>(PhantomData<E>);

/// An `EventId` uniquely identifies an event stored in a specific [`World`].
///
/// An `EventId` can among other things be used to trace the flow of an event from the point it was
/// sent to the point it was processed. `EventId`s increase monotonically by send order.
///
/// [`World`]: crate::world::World
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, Debug, PartialEq, Hash)
)]
pub struct EventId<E: Event> {
    /// Uniquely identifies the event associated with this ID.
    // This value corresponds to the order in which each event was added to the world.
    pub id: usize,
    /// The source code location that triggered this event.
    pub caller: MaybeLocation,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore, clone))]
    pub(super) _marker: PhantomData<E>,
}

impl<E: Event> Copy for EventId<E> {}

impl<E: Event> Clone for EventId<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: Event> fmt::Display for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<E: Event> fmt::Debug for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "event<{}>#{}",
            core::any::type_name::<E>().split("::").last().unwrap(),
            self.id,
        )
    }
}

impl<E: Event> PartialEq for EventId<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E: Event> Eq for EventId<E> {}

impl<E: Event> PartialOrd for EventId<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: Event> Ord for EventId<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<E: Event> Hash for EventId<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state);
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub(crate) struct EventInstance<E: Event> {
    pub event_id: EventId<E>,
    pub event: E,
}
