use bevy_ptr::{Ptr, PtrMut};

use crate::component::ComponentId;
use crate::event::Event;
use crate::observer::ObserverTrigger;
use crate::world::World;

/// The event data that an [`Observer`] is triggered with.
///
/// The provided implementations of this trait are:
///
/// - All [`Event`] types.
/// - [`DynamicEvent`], which matches any [`Event`]s dynamically added to the observer with [`Observer::with_event`] and does not reify the event data.
///
/// # Safety
///
/// Implementor must ensure that:
/// - [`EventData::init_components`] must register a [`ComponentId`] for each [`Event`] type used in the output type.
///
/// [`Observer`]: crate::observer::Observer
/// [`Observer::with_event`]: crate::observer::Observer::with_event
pub unsafe trait EventData: 'static {
    /// The item returned by this [`EventData`] that will be passed to the observer system function.
    /// Most of the time this will be a mutable reference to an [`Event`] type or a [`PtrMut`].
    type Item<'trigger>;
    /// The read-only variant of the [`Item`](EventData::Item).
    type ReadOnlyItem<'trigger>: Copy;

    /// Casts a pointer to the output [`Item`](EventData::Item) type.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the given `ptr` can be safely converted to the output [`Item`](EventData::Item) type.
    unsafe fn cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Self::Item<'trigger>;

    /// Initialize the components required by this event data.
    fn init_components(world: &mut World, ids: impl FnMut(ComponentId));

    /// Shrink the [`Item`](EventData::Item) to a shorter lifetime.
    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short>;

    /// Shrink the [`Item`](EventData::Item) to a shorter lifetime [`ReadOnlyItem`](EventData::ReadOnlyItem).
    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short>;
}

// SAFETY: The event type has a component id registered in `init_components`.
unsafe impl<E: Event> EventData for E {
    type Item<'trigger> = &'trigger mut E;
    type ReadOnlyItem<'trigger> = &'trigger E;

    unsafe fn cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Self::Item<'trigger> {
        // SAFETY: Caller must ensure that ptr can be safely cast to the Item type.
        unsafe { ptr.deref_mut() }
    }

    fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
        let id = world.init_component::<E>();
        ids(id);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        item
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        item
    }
}

/// [`EventData`] that matches any event type and performs no casting. Instead, it returns the pointer as is.
/// This is useful for observers that do not need to access the event data, or need to do so dynamically.
///
/// # Example
///
/// ## Listen to [`OnAdd`] and [`OnRemove`] events in the same observer
///
/// ```
/// # use crate::prelude::*;
/// # use bevy_ecs_macros::Component;
/// #
/// /// The component type to listen for on add and remove events.
/// #[derive(Component)]
/// struct A;
///
/// let mut world = World::new();
///
/// // Fetch the component ids for the events
/// let on_add = world.init_component::<OnAdd>();
/// let on_remove = world.init_component::<OnRemove>();
///
/// world.spawn(
///     Observer::new(|trigger: Trigger<DynamicEvent, A>| {
///         // This observer function is called for both OnAdd and OnRemove events!
///         let ptr_mut = trigger.event_mut();
///         // do something with the PtrMut, if needed
///     })
///     // Safely register the component ids for the events to the observer
///     .with_event(on_add)
///     .with_event(on_remove),
/// );
///
/// // The observer will be called twice for these two function calls:
/// let entity = world.spawn(A).id();
/// world.despawn(entity);
/// ```
///
/// [`OnAdd`]: crate::event::OnAdd
/// [`OnRemove`]: crate::event::OnRemove
pub struct DynamicEvent;

// SAFETY: Performs no unsafe operations, returns the pointer as is.
unsafe impl EventData for DynamicEvent {
    type Item<'trigger> = PtrMut<'trigger>;
    type ReadOnlyItem<'trigger> = Ptr<'trigger>;

    unsafe fn cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Self::Item<'trigger> {
        ptr
    }

    fn init_components(_world: &mut World, _ids: impl FnMut(ComponentId)) {}

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        item.reborrow()
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        item.as_ref()
    }
}

pub struct ReflectEvent;

unsafe impl EventData for ReflectEvent {
    type Item<'trigger> = &'trigger mut dyn bevy_reflect::Reflect;
    type ReadOnlyItem<'trigger> = &'trigger dyn bevy_reflect::Reflect;

    unsafe fn cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Self::Item<'trigger> {
        let type_id = world
            .components()
            .get_info(observer_trigger.event_type)
            .unwrap()
            .type_id()
            .unwrap();
        let type_registry = world.resource::<crate::reflect::AppTypeRegistry>().read();
        let reflect_from_ptr = type_registry
            .get_type_data::<bevy_reflect::ReflectFromPtr>(type_id)
            .unwrap();

        // SAFETY: The ReflectFromPtr data was fetched based on the observed event type's type id.
        unsafe { reflect_from_ptr.as_reflect_mut(ptr) }
    }

    fn init_components(world: &mut World, ids: impl FnMut(ComponentId)) {}

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        *item
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        *item
    }
}
