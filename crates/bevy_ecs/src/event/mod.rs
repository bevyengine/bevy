//! [`Event`] functionality.
mod trigger;

pub use bevy_ecs_macros::{EntityEvent, Event};
pub use trigger::*;

use crate::{
    component::{Component, ComponentId},
    entity::Entity,
    world::World,
};
use core::marker::PhantomData;

/// An [`Event`] is something that "happens" at a given moment.
///
/// To make an [`Event`] "happen", you "trigger" it on a [`World`] using [`World::trigger`] or via a [`Command`](crate::system::Command)
/// using [`Commands::trigger`](crate::system::Commands::trigger). This causes any [`Observer`](crate::observer::Observer) watching for that
/// [`Event`] to run _immediately_, as part of the [`World::trigger`] call.
///
/// First, we create an [`Event`] type, typically by deriving the trait.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// #[derive(Event)]
/// struct Speak {
///     message: String,
/// }
/// ```
///
/// Then, we add an [`Observer`](crate::observer::Observer) to watch for this event type:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Event)]
/// # struct Speak {
/// #     message: String,
/// # }
/// #
/// # let mut world = World::new();
/// #
/// world.add_observer(|speak: On<Speak>| {
///     println!("{}", speak.message);
/// });
/// ```
///
/// Finally, we trigger the event by calling [`World::trigger`](World::trigger):
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Event)]
/// # struct Speak {
/// #     message: String,
/// # }
/// #
/// # let mut world = World::new();
/// #
/// # world.add_observer(|speak: On<Speak>| {
/// #     println!("{}", speak.message);
/// # });
/// #
/// # world.flush();
/// #
/// world.trigger(Speak {
///     message: "Hello!".to_string(),
/// });
/// ```
///
/// # Triggers
///
/// Every [`Event`] has an associated [`Trigger`] implementation (set via [`Event::Trigger`]), which defines which observers will run,
/// what data will be passed to them, and the order they will be run in. Unless you are an internals developer or you have very specific
/// needs, you don't need to worry too much about [`Trigger`]. When you derive [`Event`] (or a more specific event trait like [`EntityEvent`]),
/// a [`Trigger`] will be provided for you.
///
/// The [`Event`] derive defaults [`Event::Trigger`] to [`GlobalTrigger`], which will run all observers that watch for the [`Event`].
///
/// # Entity Events
///
/// For events that "target" a specific [`Entity`], see [`EntityEvent`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Event`",
    label = "invalid `Event`",
    note = "consider annotating `{Self}` with `#[derive(Event)]`"
)]
pub trait Event: Send + Sync + Sized + 'static {
    /// Defines which observers will run, what data will be passed to them, and the order they will be run in. See [`Trigger`] for more info.
    type Trigger<'a>: Trigger<Self>;
}

/// An [`EntityEvent`] is an [`Event`] that is triggered for a specific [`EntityEvent::event_target`] entity:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let entity = world.spawn_empty().id();
/// #[derive(EntityEvent)]
/// struct Explode {
///     entity: Entity,
/// }
///
/// world.add_observer(|event: On<Explode>, mut commands: Commands| {
///     println!("Entity {} goes BOOM!", event.entity);
///     commands.entity(event.entity).despawn();
/// });
///
/// world.trigger(Explode { entity });
/// ```
///
/// [`EntityEvent`] will set [`EntityEvent::event_target`] automatically for named structs with an `entity` field name (as seen above). It also works for tuple structs
/// whose only field is [`Entity`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(EntityEvent)]
/// struct Explode(Entity);
/// ```
///
/// The [`EntityEvent::event_target`] can also be manually set using the `#[event_target]` field attribute:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(EntityEvent)]
/// struct Explode {
///     #[event_target]
///     exploded_entity: Entity,
/// }
/// ```
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(EntityEvent)]
/// struct Explode(#[event_target] Entity);
/// ```
///
/// ## Trigger Behavior
///
/// When derived, [`EntityEvent`] defaults to setting [`Event::Trigger`] to [`EntityTrigger`], which will run all normal "untargeted"
/// observers added via [`World::add_observer`], just like a default [`Event`] would (see the example above).
///
/// However it will _also_ run all observers that watch _specific_ entities, which enables you to assign entity-specific logic:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component, Debug)]
/// # struct Name(String);
/// # let mut world = World::default();
/// # let e1 = world.spawn_empty().id();
/// # let e2 = world.spawn_empty().id();
/// # #[derive(EntityEvent)]
/// # struct Explode {
/// #    entity: Entity,
/// # }
/// world.entity_mut(e1).observe(|event: On<Explode>, mut commands: Commands| {
///     println!("Boom!");
///     commands.entity(event.entity).despawn();
/// });
///
/// world.entity_mut(e2).observe(|event: On<Explode>, mut commands: Commands| {
///     println!("The explosion fizzles! This entity is immune!");
/// });
/// ```
///
/// ## [`EntityEvent`] Propagation
///
/// When deriving [`EntityEvent`], you can enable "event propagation" (also known as "event bubbling") by
/// specifying the `#[entity_event(propagate)]` attribute:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(EntityEvent)]
/// #[entity_event(propagate)]
/// struct Click {
///     entity: Entity,
/// }
/// ```
///
/// This will default to using the [`ChildOf`](crate::hierarchy::ChildOf) component to propagate the [`Event`] "up"
/// the hierarchy (from child to parent).
///
/// You can also specify your own [`Traversal`](crate::traversal::Traversal) implementation. A common pattern is to use
/// [`Relationship`](crate::relationship::Relationship) components, which will follow the relationships to their root
/// (just be sure to avoid cycles ... these aren't detected for performance reasons):
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// #[relationship(relationship_target = ClickableBy)]
/// struct Clickable(Entity);
///
/// #[derive(Component)]
/// #[relationship_target(relationship = Clickable)]
/// struct ClickableBy(Vec<Entity>);
///
/// #[derive(EntityEvent)]
/// #[entity_event(propagate = &'static Clickable)]
/// struct Click {
///     entity: Entity,
/// }
/// ```
///
/// By default, propagation requires observers to opt-in:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(EntityEvent)]
/// #[entity_event(propagate)]
/// struct Click {
///     entity: Entity,
/// }
///
/// # let mut world = World::default();
/// world.add_observer(|mut click: On<Click>| {
///   // this will propagate the event up to the parent, using `ChildOf`
///   click.propagate(true);
/// });
/// ```
///
/// But you can enable auto propagation using the `#[entity_event(auto_propagate)]` attribute:
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(EntityEvent)]
/// #[entity_event(propagate, auto_propagate)]
/// struct Click {
///     entity: Entity,
/// }
/// ```
///
/// You can also _stop_ propagation like this:
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(EntityEvent)]
/// # #[entity_event(propagate)]
/// # struct Click {
/// #    entity: Entity,
/// # }
/// # fn is_finished_propagating() -> bool { true }
/// # let mut world = World::default();
/// world.add_observer(|mut click: On<Click>| {
///   if is_finished_propagating() {
///     click.propagate(false);
///   }
/// });
/// ```
///
/// ## Naming and Usage Conventions
///
/// In most cases, it is recommended to use a named struct field for the "event target" entity, and to use
/// a name that is descriptive as possible, as this makes events easier to understand and read.
///
/// For events with only one [`Entity`] field, `entity` is often a reasonable name. But if there are multiple
/// [`Entity`] fields, it is often a good idea to use a more descriptive name.
///
/// It is also generally recommended to _consume_ "event target" entities directly via their named field, as this
/// can make the context clearer, allows for more specific documentation hints in IDEs, and it generally reads better.
///
/// ## Manually spawning [`EntityEvent`] observers
///
/// The examples above that call [`EntityWorldMut::observe`] to add entity-specific observer logic are
/// just shorthand for spawning an [`Observer`] directly and manually watching the entity:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let entity = world.spawn_empty().id();
/// # #[derive(EntityEvent)]
/// # struct Explode(Entity);
/// let mut observer = Observer::new(|event: On<Explode>| {});
/// observer.watch_entity(entity);
/// world.spawn(observer);
/// ```
///
/// Note that the [`Observer`] component is not added to the entity it is observing. Observers should always be their own entities, as there
/// can be multiple observers of the same entity!
///
/// You can call [`Observer::watch_entity`] more than once or [`Observer::watch_entities`] to watch multiple entities with the same [`Observer`].
///
/// [`EntityWorldMut::observe`]: crate::world::EntityWorldMut::observe
/// [`Observer`]: crate::observer::Observer
/// [`Observer::watch_entity`]: crate::observer::Observer::watch_entity
/// [`Observer::watch_entities`]: crate::observer::Observer::watch_entities
pub trait EntityEvent: Event {
    /// The [`Entity`] "target" of this [`EntityEvent`]. When triggered, this will run observers that watch for this specific entity.
    fn event_target(&self) -> Entity;
    /// Returns a mutable reference to the [`Entity`] "target" of this [`EntityEvent`]. When triggered, this will run observers that watch for this specific entity.
    ///
    /// Note: In general, this should not be mutated from within an [`Observer`](crate::observer::Observer), as this will not "retarget"
    /// the event in any of Bevy's built-in [`Trigger`] implementations.
    fn event_target_mut(&mut self) -> &mut Entity;
}

impl World {
    /// Generates the [`EventKey`] for this event type.
    ///
    /// If this type has already been registered,
    /// this will return the existing [`EventKey`].
    ///
    /// This is used by various dynamically typed observer APIs,
    /// such as [`DeferredWorld::trigger_raw`](crate::world::DeferredWorld::trigger_raw).
    pub fn register_event_key<E: Event>(&mut self) -> EventKey {
        EventKey(self.register_component::<EventWrapperComponent<E>>())
    }

    /// Fetches the [`EventKey`] for this event type,
    /// if it has already been generated.
    ///
    /// This is used by various dynamically typed observer APIs,
    /// such as [`DeferredWorld::trigger_raw`](crate::world::DeferredWorld::trigger_raw).
    pub fn event_key<E: Event>(&self) -> Option<EventKey> {
        self.component_id::<EventWrapperComponent<E>>()
            .map(EventKey)
    }
}

/// An internal type that implements [`Component`] for a given [`Event`] type.
///
/// This exists so we can easily get access to a unique [`ComponentId`] for each [`Event`] type,
/// without requiring that [`Event`] types implement [`Component`] directly.
/// [`ComponentId`] is used internally as a unique identifier for events because they are:
///
/// - Unique to each event type.
/// - Can be quickly generated and looked up.
/// - Are compatible with dynamic event types, which aren't backed by a Rust type.
///
/// This type is an implementation detail and should never be made public.
// TODO: refactor events to store their metadata on distinct entities, rather than using `ComponentId`
#[derive(Component)]
struct EventWrapperComponent<E: Event>(PhantomData<E>);

/// A unique identifier for an [`Event`], used by [observers].
///
/// You can look up the key for your event by calling the [`World::event_key`] method.
///
/// [observers]: crate::observer
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct EventKey(pub(crate) ComponentId);

/// This is deprecated. See [`MessageCursor`](crate::message::MessageCursor)
#[deprecated(since = "0.17.0", note = "Renamed to `MessageCursor`.")]
pub type EventCursor<E> = crate::message::MessageCursor<E>;

/// This is deprecated. See [`MessageMutator`](crate::message::MessageMutator)
#[deprecated(since = "0.17.0", note = "Renamed to `MessageMutator`.")]
pub type EventMutator<'w, 's, E> = crate::message::MessageMutator<'w, 's, E>;

/// This is deprecated. See [`MessageReader`](crate::message::MessageReader)
#[deprecated(since = "0.17.0", note = "Renamed to `MessageReader`.")]
pub type EventReader<'w, 's, E> = crate::message::MessageReader<'w, 's, E>;

/// This is deprecated. See [`MessageWriter`](crate::message::MessageWriter)
#[deprecated(since = "0.17.0", note = "Renamed to `MessageWriter`.")]
pub type EventWriter<'w, E> = crate::message::MessageWriter<'w, E>;

/// This is deprecated. See [`Messages`](crate::message::Messages)
#[deprecated(since = "0.17.0", note = "Renamed to `Messages`.")]
pub type Events<E> = crate::message::Messages<E>;

/// This is deprecated. See [`MessageIterator`](crate::message::MessageIterator)
#[deprecated(since = "0.17.0", note = "Renamed to `MessageIterator`.")]
pub type EventIterator<'a, E> = crate::message::MessageIterator<'a, E>;

/// This is deprecated. See [`MessageMutIterator`](crate::message::MessageMutIterator)
#[deprecated(since = "0.17.0", note = "Renamed to `MessageIterator`.")]
pub type EventMutIterator<'a, E> = crate::message::MessageMutIterator<'a, E>;

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};
    use bevy_ecs::{message::*, system::assert_is_read_only_system};
    use bevy_ecs_macros::Message;

    #[derive(Message, Copy, Clone, PartialEq, Eq, Debug)]
    struct TestEvent {
        i: usize,
    }

    #[derive(Message, Clone, PartialEq, Debug, Default)]
    struct EmptyTestEvent;

    fn get_events<E: Message + Clone>(
        events: &Messages<E>,
        cursor: &mut MessageCursor<E>,
    ) -> Vec<E> {
        cursor.read(events).cloned().collect::<Vec<E>>()
    }

    #[test]
    fn test_events() {
        let mut events = Messages::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        // this reader will miss event_0 and event_1 because it wont read them over the course of
        // two updates
        let mut reader_missed: MessageCursor<TestEvent> = events.get_cursor();

        let mut reader_a: MessageCursor<TestEvent> = events.get_cursor();

        events.write(event_0);

        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![event_0],
            "reader_a created before event receives event"
        );
        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![],
            "second iteration of reader_a created before event results in zero events"
        );

        let mut reader_b: MessageCursor<TestEvent> = events.get_cursor();

        assert_eq!(
            get_events(&events, &mut reader_b),
            vec![event_0],
            "reader_b created after event receives event"
        );
        assert_eq!(
            get_events(&events, &mut reader_b),
            vec![],
            "second iteration of reader_b created after event results in zero events"
        );

        events.write(event_1);

        let mut reader_c = events.get_cursor();

        assert_eq!(
            get_events(&events, &mut reader_c),
            vec![event_0, event_1],
            "reader_c created after two events receives both events"
        );
        assert_eq!(
            get_events(&events, &mut reader_c),
            vec![],
            "second iteration of reader_c created after two event results in zero events"
        );

        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![event_1],
            "reader_a receives next unread event"
        );

        events.update();

        let mut reader_d = events.get_cursor();

        events.write(event_2);

        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![event_2],
            "reader_a receives event created after update"
        );
        assert_eq!(
            get_events(&events, &mut reader_b),
            vec![event_1, event_2],
            "reader_b receives events created before and after update"
        );
        assert_eq!(
            get_events(&events, &mut reader_d),
            vec![event_0, event_1, event_2],
            "reader_d receives all events created before and after update"
        );

        events.update();

        assert_eq!(
            get_events(&events, &mut reader_missed),
            vec![event_2],
            "reader_missed missed events unread after two update() calls"
        );
    }

    // Events Collection
    fn events_clear_and_read_impl(clear_func: impl FnOnce(&mut Messages<TestEvent>)) {
        let mut events = Messages::<TestEvent>::default();
        let mut reader = events.get_cursor();

        assert!(reader.read(&events).next().is_none());

        events.write(TestEvent { i: 0 });
        assert_eq!(*reader.read(&events).next().unwrap(), TestEvent { i: 0 });
        assert_eq!(reader.read(&events).next(), None);

        events.write(TestEvent { i: 1 });
        clear_func(&mut events);
        assert!(reader.read(&events).next().is_none());

        events.write(TestEvent { i: 2 });
        events.update();
        events.write(TestEvent { i: 3 });

        assert!(reader
            .read(&events)
            .eq([TestEvent { i: 2 }, TestEvent { i: 3 }].iter()));
    }

    #[test]
    fn test_events_clear_and_read() {
        events_clear_and_read_impl(Messages::clear);
    }

    #[test]
    fn test_events_drain_and_read() {
        events_clear_and_read_impl(|events| {
            assert!(events
                .drain()
                .eq(vec![TestEvent { i: 0 }, TestEvent { i: 1 }].into_iter()));
        });
    }

    #[test]
    fn test_events_write_default() {
        let mut events = Messages::<EmptyTestEvent>::default();
        events.write_default();

        let mut reader = events.get_cursor();
        assert_eq!(get_events(&events, &mut reader), vec![EmptyTestEvent]);
    }

    #[test]
    fn test_write_events_ids() {
        let mut events = Messages::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        let event_0_id = events.write(event_0);

        assert_eq!(
            events.get_message(event_0_id.id),
            Some((&event_0, event_0_id)),
            "Getting a sent event by ID should return the original event"
        );

        let mut event_ids = events.write_batch([event_1, event_2]);

        let event_id = event_ids.next().expect("Event 1 must have been sent");

        assert_eq!(
            events.get_message(event_id.id),
            Some((&event_1, event_id)),
            "Getting a sent event by ID should return the original event"
        );

        let event_id = event_ids.next().expect("Event 2 must have been sent");

        assert_eq!(
            events.get_message(event_id.id),
            Some((&event_2, event_id)),
            "Getting a sent event by ID should return the original event"
        );

        assert!(
            event_ids.next().is_none(),
            "Only sent two events; got more than two IDs"
        );
    }

    #[test]
    fn test_event_registry_can_add_and_remove_events_to_world() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        MessageRegistry::register_message::<TestEvent>(&mut world);

        let has_events = world.get_resource::<Messages<TestEvent>>().is_some();
        assert!(has_events, "Should have the events resource");

        MessageRegistry::deregister_messages::<TestEvent>(&mut world);

        let has_events = world.get_resource::<Messages<TestEvent>>().is_some();
        assert!(!has_events, "Should not have the events resource");
    }

    #[test]
    fn test_events_update_drain() {
        let mut events = Messages::<TestEvent>::default();
        let mut reader = events.get_cursor();

        events.write(TestEvent { i: 0 });
        events.write(TestEvent { i: 1 });
        assert_eq!(reader.read(&events).count(), 2);

        let mut old_events = Vec::from_iter(events.update_drain());
        assert!(old_events.is_empty());

        events.write(TestEvent { i: 2 });
        assert_eq!(reader.read(&events).count(), 1);

        old_events.extend(events.update_drain());
        assert_eq!(old_events.len(), 2);

        old_events.extend(events.update_drain());
        assert_eq!(
            old_events,
            &[TestEvent { i: 0 }, TestEvent { i: 1 }, TestEvent { i: 2 }]
        );
    }

    #[test]
    fn test_events_empty() {
        let mut events = Messages::<TestEvent>::default();
        assert!(events.is_empty());

        events.write(TestEvent { i: 0 });
        assert!(!events.is_empty());

        events.update();
        assert!(!events.is_empty());

        // events are only empty after the second call to update
        // due to double buffering.
        events.update();
        assert!(events.is_empty());
    }

    #[test]
    fn test_events_extend_impl() {
        let mut events = Messages::<TestEvent>::default();
        let mut reader = events.get_cursor();

        events.extend(vec![TestEvent { i: 0 }, TestEvent { i: 1 }]);
        assert!(reader
            .read(&events)
            .eq([TestEvent { i: 0 }, TestEvent { i: 1 }].iter()));
    }

    // Cursor
    #[test]
    fn test_event_cursor_read() {
        let mut events = Messages::<TestEvent>::default();
        let mut cursor = events.get_cursor();
        assert!(cursor.read(&events).next().is_none());

        events.write(TestEvent { i: 0 });
        let sent_event = cursor.read(&events).next().unwrap();
        assert_eq!(sent_event, &TestEvent { i: 0 });
        assert!(cursor.read(&events).next().is_none());

        events.write(TestEvent { i: 2 });
        let sent_event = cursor.read(&events).next().unwrap();
        assert_eq!(sent_event, &TestEvent { i: 2 });
        assert!(cursor.read(&events).next().is_none());

        events.clear();
        assert!(cursor.read(&events).next().is_none());
    }

    #[test]
    fn test_event_cursor_read_mut() {
        let mut events = Messages::<TestEvent>::default();
        let mut write_cursor = events.get_cursor();
        let mut read_cursor = events.get_cursor();
        assert!(write_cursor.read_mut(&mut events).next().is_none());
        assert!(read_cursor.read(&events).next().is_none());

        events.write(TestEvent { i: 0 });
        let sent_event = write_cursor.read_mut(&mut events).next().unwrap();
        assert_eq!(sent_event, &mut TestEvent { i: 0 });
        *sent_event = TestEvent { i: 1 }; // Mutate whole event
        assert_eq!(
            read_cursor.read(&events).next().unwrap(),
            &TestEvent { i: 1 }
        );
        assert!(read_cursor.read(&events).next().is_none());

        events.write(TestEvent { i: 2 });
        let sent_event = write_cursor.read_mut(&mut events).next().unwrap();
        assert_eq!(sent_event, &mut TestEvent { i: 2 });
        sent_event.i = 3; // Mutate sub value
        assert_eq!(
            read_cursor.read(&events).next().unwrap(),
            &TestEvent { i: 3 }
        );
        assert!(read_cursor.read(&events).next().is_none());

        events.clear();
        assert!(write_cursor.read(&events).next().is_none());
        assert!(read_cursor.read(&events).next().is_none());
    }

    #[test]
    fn test_event_cursor_clear() {
        let mut events = Messages::<TestEvent>::default();
        let mut reader = events.get_cursor();

        events.write(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 1);
        reader.clear(&events);
        assert_eq!(reader.len(&events), 0);
    }

    #[test]
    fn test_event_cursor_len_update() {
        let mut events = Messages::<TestEvent>::default();
        events.write(TestEvent { i: 0 });
        events.write(TestEvent { i: 0 });
        let reader = events.get_cursor();
        assert_eq!(reader.len(&events), 2);
        events.update();
        events.write(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 3);
        events.update();
        assert_eq!(reader.len(&events), 1);
        events.update();
        assert!(reader.is_empty(&events));
    }

    #[test]
    fn test_event_cursor_len_current() {
        let mut events = Messages::<TestEvent>::default();
        events.write(TestEvent { i: 0 });
        let reader = events.get_cursor_current();
        assert!(reader.is_empty(&events));
        events.write(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 1);
        assert!(!reader.is_empty(&events));
    }

    #[test]
    fn test_event_cursor_iter_len_updated() {
        let mut events = Messages::<TestEvent>::default();
        events.write(TestEvent { i: 0 });
        events.write(TestEvent { i: 1 });
        events.write(TestEvent { i: 2 });
        let mut reader = events.get_cursor();
        let mut iter = reader.read(&events);
        assert_eq!(iter.len(), 3);
        iter.next();
        assert_eq!(iter.len(), 2);
        iter.next();
        assert_eq!(iter.len(), 1);
        iter.next();
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn test_event_cursor_len_empty() {
        let events = Messages::<TestEvent>::default();
        assert_eq!(events.get_cursor().len(&events), 0);
        assert!(events.get_cursor().is_empty(&events));
    }

    #[test]
    fn test_event_cursor_len_filled() {
        let mut events = Messages::<TestEvent>::default();
        events.write(TestEvent { i: 0 });
        assert_eq!(events.get_cursor().len(&events), 1);
        assert!(!events.get_cursor().is_empty(&events));
    }

    #[cfg(feature = "multi_threaded")]
    #[test]
    fn test_event_cursor_par_read() {
        use crate::prelude::*;
        use core::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Resource)]
        struct Counter(AtomicUsize);

        let mut world = World::new();
        world.init_resource::<Messages<TestEvent>>();
        for _ in 0..100 {
            world.write_message(TestEvent { i: 1 });
        }

        let mut schedule = Schedule::default();

        schedule.add_systems(
            |mut cursor: Local<MessageCursor<TestEvent>>,
             events: Res<Messages<TestEvent>>,
             counter: ResMut<Counter>| {
                cursor.par_read(&events).for_each(|event| {
                    counter.0.fetch_add(event.i, Ordering::Relaxed);
                });
            },
        );

        world.insert_resource(Counter(AtomicUsize::new(0)));
        schedule.run(&mut world);
        let counter = world.remove_resource::<Counter>().unwrap();
        assert_eq!(counter.0.into_inner(), 100);

        world.insert_resource(Counter(AtomicUsize::new(0)));
        schedule.run(&mut world);
        let counter = world.remove_resource::<Counter>().unwrap();
        assert_eq!(
            counter.0.into_inner(),
            0,
            "par_read should have consumed events but didn't"
        );
    }

    #[cfg(feature = "multi_threaded")]
    #[test]
    fn test_event_cursor_par_read_mut() {
        use crate::prelude::*;
        use core::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Resource)]
        struct Counter(AtomicUsize);

        let mut world = World::new();
        world.init_resource::<Messages<TestEvent>>();
        for _ in 0..100 {
            world.write_message(TestEvent { i: 1 });
        }
        let mut schedule = Schedule::default();
        schedule.add_systems(
            |mut cursor: Local<MessageCursor<TestEvent>>,
             mut events: ResMut<Messages<TestEvent>>,
             counter: ResMut<Counter>| {
                cursor.par_read_mut(&mut events).for_each(|event| {
                    event.i += 1;
                    counter.0.fetch_add(event.i, Ordering::Relaxed);
                });
            },
        );
        world.insert_resource(Counter(AtomicUsize::new(0)));
        schedule.run(&mut world);
        let counter = world.remove_resource::<Counter>().unwrap();
        assert_eq!(counter.0.into_inner(), 200, "Initial run failed");

        world.insert_resource(Counter(AtomicUsize::new(0)));
        schedule.run(&mut world);
        let counter = world.remove_resource::<Counter>().unwrap();
        assert_eq!(
            counter.0.into_inner(),
            0,
            "par_read_mut should have consumed events but didn't"
        );
    }

    // Reader & Mutator
    #[test]
    fn ensure_reader_readonly() {
        fn reader_system(_: MessageReader<EmptyTestEvent>) {}

        assert_is_read_only_system(reader_system);
    }

    #[test]
    fn test_event_reader_iter_last() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Messages<TestEvent>>();

        let mut reader = IntoSystem::into_system(
            |mut events: MessageReader<TestEvent>| -> Option<TestEvent> {
                events.read().last().copied()
            },
        );
        reader.initialize(&mut world);

        let last = reader.run((), &mut world).unwrap();
        assert!(last.is_none(), "MessageReader should be empty");

        world.write_message(TestEvent { i: 0 });
        let last = reader.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 0 }));

        world.write_message(TestEvent { i: 1 });
        world.write_message(TestEvent { i: 2 });
        world.write_message(TestEvent { i: 3 });
        let last = reader.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 3 }));

        let last = reader.run((), &mut world).unwrap();
        assert!(last.is_none(), "MessageReader should be empty");
    }

    #[test]
    fn test_event_mutator_iter_last() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Messages<TestEvent>>();

        let mut mutator = IntoSystem::into_system(
            |mut events: MessageMutator<TestEvent>| -> Option<TestEvent> {
                events.read().last().copied()
            },
        );
        mutator.initialize(&mut world);

        let last = mutator.run((), &mut world).unwrap();
        assert!(last.is_none(), "EventMutator should be empty");

        world.write_message(TestEvent { i: 0 });
        let last = mutator.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 0 }));

        world.write_message(TestEvent { i: 1 });
        world.write_message(TestEvent { i: 2 });
        world.write_message(TestEvent { i: 3 });
        let last = mutator.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 3 }));

        let last = mutator.run((), &mut world).unwrap();
        assert!(last.is_none(), "EventMutator should be empty");
    }

    #[test]
    fn test_event_reader_iter_nth() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Messages<TestEvent>>();

        world.write_message(TestEvent { i: 0 });
        world.write_message(TestEvent { i: 1 });
        world.write_message(TestEvent { i: 2 });
        world.write_message(TestEvent { i: 3 });
        world.write_message(TestEvent { i: 4 });

        let mut schedule = Schedule::default();
        schedule.add_systems(|mut events: MessageReader<TestEvent>| {
            let mut iter = events.read();

            assert_eq!(iter.next(), Some(&TestEvent { i: 0 }));
            assert_eq!(iter.nth(2), Some(&TestEvent { i: 3 }));
            assert_eq!(iter.nth(1), None);

            assert!(events.is_empty());
        });
        schedule.run(&mut world);
    }

    #[test]
    fn test_event_mutator_iter_nth() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Messages<TestEvent>>();

        world.write_message(TestEvent { i: 0 });
        world.write_message(TestEvent { i: 1 });
        world.write_message(TestEvent { i: 2 });
        world.write_message(TestEvent { i: 3 });
        world.write_message(TestEvent { i: 4 });

        let mut schedule = Schedule::default();
        schedule.add_systems(|mut events: MessageReader<TestEvent>| {
            let mut iter = events.read();

            assert_eq!(iter.next(), Some(&TestEvent { i: 0 }));
            assert_eq!(iter.nth(2), Some(&TestEvent { i: 3 }));
            assert_eq!(iter.nth(1), None);

            assert!(events.is_empty());
        });
        schedule.run(&mut world);
    }
}
