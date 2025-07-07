//! Event handling types.
mod base;
mod collections;
mod event_cursor;
mod iterators;
mod mut_iterators;
mod mutator;
mod reader;
mod registry;
mod update;
mod writer;

pub(crate) use base::EventInstance;
pub use base::{BufferedEvent, EntityEvent, Event, EventId, EventKey};
pub use bevy_ecs_macros::{BufferedEvent, EntityEvent, Event};
#[expect(deprecated, reason = "`SendBatchIds` was renamed to `WriteBatchIds`.")]
pub use collections::{Events, SendBatchIds, WriteBatchIds};
pub use event_cursor::EventCursor;
#[cfg(feature = "multi_threaded")]
pub use iterators::EventParIter;
pub use iterators::{EventIterator, EventIteratorWithId};
#[cfg(feature = "multi_threaded")]
pub use mut_iterators::EventMutParIter;
pub use mut_iterators::{EventMutIterator, EventMutIteratorWithId};
pub use mutator::EventMutator;
pub use reader::EventReader;
pub use registry::{EventRegistry, ShouldUpdateEvents};
#[expect(
    deprecated,
    reason = "`EventUpdates` was renamed to `EventUpdateSystems`."
)]
pub use update::{
    event_update_condition, event_update_system, signal_event_update_system, EventUpdateSystems,
    EventUpdates,
};
pub use writer::EventWriter;

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};
    use bevy_ecs::{event::*, system::assert_is_read_only_system};
    use bevy_ecs_macros::BufferedEvent;

    #[derive(Event, BufferedEvent, Copy, Clone, PartialEq, Eq, Debug)]
    struct TestEvent {
        i: usize,
    }

    #[derive(Event, BufferedEvent, Clone, PartialEq, Debug, Default)]
    struct EmptyTestEvent;

    fn get_events<E: BufferedEvent + Clone>(
        events: &Events<E>,
        cursor: &mut EventCursor<E>,
    ) -> Vec<E> {
        cursor.read(events).cloned().collect::<Vec<E>>()
    }

    #[test]
    fn test_events() {
        let mut events = Events::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        // this reader will miss event_0 and event_1 because it wont read them over the course of
        // two updates
        let mut reader_missed: EventCursor<TestEvent> = events.get_cursor();

        let mut reader_a: EventCursor<TestEvent> = events.get_cursor();

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

        let mut reader_b: EventCursor<TestEvent> = events.get_cursor();

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
    fn events_clear_and_read_impl(clear_func: impl FnOnce(&mut Events<TestEvent>)) {
        let mut events = Events::<TestEvent>::default();
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
        events_clear_and_read_impl(Events::clear);
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
        let mut events = Events::<EmptyTestEvent>::default();
        events.write_default();

        let mut reader = events.get_cursor();
        assert_eq!(get_events(&events, &mut reader), vec![EmptyTestEvent]);
    }

    #[test]
    fn test_write_events_ids() {
        let mut events = Events::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        let event_0_id = events.write(event_0);

        assert_eq!(
            events.get_event(event_0_id.id),
            Some((&event_0, event_0_id)),
            "Getting a sent event by ID should return the original event"
        );

        let mut event_ids = events.write_batch([event_1, event_2]);

        let event_id = event_ids.next().expect("Event 1 must have been sent");

        assert_eq!(
            events.get_event(event_id.id),
            Some((&event_1, event_id)),
            "Getting a sent event by ID should return the original event"
        );

        let event_id = event_ids.next().expect("Event 2 must have been sent");

        assert_eq!(
            events.get_event(event_id.id),
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
        EventRegistry::register_event::<TestEvent>(&mut world);

        let has_events = world.get_resource::<Events<TestEvent>>().is_some();
        assert!(has_events, "Should have the events resource");

        EventRegistry::deregister_events::<TestEvent>(&mut world);

        let has_events = world.get_resource::<Events<TestEvent>>().is_some();
        assert!(!has_events, "Should not have the events resource");
    }

    #[test]
    fn test_events_update_drain() {
        let mut events = Events::<TestEvent>::default();
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
        let mut events = Events::<TestEvent>::default();
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
        let mut events = Events::<TestEvent>::default();
        let mut reader = events.get_cursor();

        events.extend(vec![TestEvent { i: 0 }, TestEvent { i: 1 }]);
        assert!(reader
            .read(&events)
            .eq([TestEvent { i: 0 }, TestEvent { i: 1 }].iter()));
    }

    // Cursor
    #[test]
    fn test_event_cursor_read() {
        let mut events = Events::<TestEvent>::default();
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
        let mut events = Events::<TestEvent>::default();
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
        let mut events = Events::<TestEvent>::default();
        let mut reader = events.get_cursor();

        events.write(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 1);
        reader.clear(&events);
        assert_eq!(reader.len(&events), 0);
    }

    #[test]
    fn test_event_cursor_len_update() {
        let mut events = Events::<TestEvent>::default();
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
        let mut events = Events::<TestEvent>::default();
        events.write(TestEvent { i: 0 });
        let reader = events.get_cursor_current();
        assert!(reader.is_empty(&events));
        events.write(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 1);
        assert!(!reader.is_empty(&events));
    }

    #[test]
    fn test_event_cursor_iter_len_updated() {
        let mut events = Events::<TestEvent>::default();
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
        let events = Events::<TestEvent>::default();
        assert_eq!(events.get_cursor().len(&events), 0);
        assert!(events.get_cursor().is_empty(&events));
    }

    #[test]
    fn test_event_cursor_len_filled() {
        let mut events = Events::<TestEvent>::default();
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
        world.init_resource::<Events<TestEvent>>();
        for _ in 0..100 {
            world.write_event(TestEvent { i: 1 });
        }

        let mut schedule = Schedule::default();

        schedule.add_systems(
            |mut cursor: Local<EventCursor<TestEvent>>,
             events: Res<Events<TestEvent>>,
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
        world.init_resource::<Events<TestEvent>>();
        for _ in 0..100 {
            world.write_event(TestEvent { i: 1 });
        }
        let mut schedule = Schedule::default();
        schedule.add_systems(
            |mut cursor: Local<EventCursor<TestEvent>>,
             mut events: ResMut<Events<TestEvent>>,
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
        fn reader_system(_: EventReader<EmptyTestEvent>) {}

        assert_is_read_only_system(reader_system);
    }

    #[test]
    fn test_event_reader_iter_last() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();

        let mut reader =
            IntoSystem::into_system(|mut events: EventReader<TestEvent>| -> Option<TestEvent> {
                events.read().last().copied()
            });
        reader.initialize(&mut world);

        let last = reader.run((), &mut world).unwrap();
        assert!(last.is_none(), "EventReader should be empty");

        world.write_event(TestEvent { i: 0 });
        let last = reader.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 0 }));

        world.write_event(TestEvent { i: 1 });
        world.write_event(TestEvent { i: 2 });
        world.write_event(TestEvent { i: 3 });
        let last = reader.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 3 }));

        let last = reader.run((), &mut world).unwrap();
        assert!(last.is_none(), "EventReader should be empty");
    }

    #[test]
    fn test_event_mutator_iter_last() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();

        let mut mutator =
            IntoSystem::into_system(|mut events: EventMutator<TestEvent>| -> Option<TestEvent> {
                events.read().last().copied()
            });
        mutator.initialize(&mut world);

        let last = mutator.run((), &mut world).unwrap();
        assert!(last.is_none(), "EventMutator should be empty");

        world.write_event(TestEvent { i: 0 });
        let last = mutator.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 0 }));

        world.write_event(TestEvent { i: 1 });
        world.write_event(TestEvent { i: 2 });
        world.write_event(TestEvent { i: 3 });
        let last = mutator.run((), &mut world).unwrap();
        assert_eq!(last, Some(TestEvent { i: 3 }));

        let last = mutator.run((), &mut world).unwrap();
        assert!(last.is_none(), "EventMutator should be empty");
    }

    #[test]
    fn test_event_reader_iter_nth() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();

        world.write_event(TestEvent { i: 0 });
        world.write_event(TestEvent { i: 1 });
        world.write_event(TestEvent { i: 2 });
        world.write_event(TestEvent { i: 3 });
        world.write_event(TestEvent { i: 4 });

        let mut schedule = Schedule::default();
        schedule.add_systems(|mut events: EventReader<TestEvent>| {
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
        world.init_resource::<Events<TestEvent>>();

        world.write_event(TestEvent { i: 0 });
        world.write_event(TestEvent { i: 1 });
        world.write_event(TestEvent { i: 2 });
        world.write_event(TestEvent { i: 3 });
        world.write_event(TestEvent { i: 4 });

        let mut schedule = Schedule::default();
        schedule.add_systems(|mut events: EventReader<TestEvent>| {
            let mut iter = events.read();

            assert_eq!(iter.next(), Some(&TestEvent { i: 0 }));
            assert_eq!(iter.nth(2), Some(&TestEvent { i: 3 }));
            assert_eq!(iter.nth(1), None);

            assert!(events.is_empty());
        });
        schedule.run(&mut world);
    }
}
