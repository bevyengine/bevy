//! Event handling types.
mod base;
mod collections;
mod iterators;
mod reader;
mod registry;
mod update;
mod writer;

pub(crate) use base::EventInstance;
pub use base::{Event, EventId};
pub use bevy_ecs_macros::Event;
pub use collections::{Events, SendBatchIds};
pub use iterators::{EventIterator, EventIteratorWithId, EventParIter};
pub use reader::{EventReader, ManualEventReader};
pub use registry::{EventRegistry, ShouldUpdateEvents};
pub use update::{
    event_update_condition, event_update_system, signal_event_update_system, EventUpdates,
};
pub use writer::EventWriter;

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use bevy_ecs::{event::*, system::assert_is_read_only_system};
    use bevy_ecs_macros::Event;

    #[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
    struct TestEvent {
        i: usize,
    }

    #[test]
    fn test_events() {
        let mut events = Events::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        // this reader will miss event_0 and event_1 because it wont read them over the course of
        // two updates
        let mut reader_missed: ManualEventReader<TestEvent> = events.get_reader();

        let mut reader_a: ManualEventReader<TestEvent> = events.get_reader();

        events.send(event_0);

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

        let mut reader_b: ManualEventReader<TestEvent> = events.get_reader();

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

        events.send(event_1);

        let mut reader_c = events.get_reader();

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

        let mut reader_d = events.get_reader();

        events.send(event_2);

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

    fn get_events<E: Event + Clone>(
        events: &Events<E>,
        reader: &mut ManualEventReader<E>,
    ) -> Vec<E> {
        reader.read(events).cloned().collect::<Vec<E>>()
    }

    #[derive(Event, PartialEq, Eq, Debug)]
    struct E(usize);

    fn events_clear_and_read_impl(clear_func: impl FnOnce(&mut Events<E>)) {
        let mut events = Events::<E>::default();
        let mut reader = events.get_reader();

        assert!(reader.read(&events).next().is_none());

        events.send(E(0));
        assert_eq!(*reader.read(&events).next().unwrap(), E(0));
        assert_eq!(reader.read(&events).next(), None);

        events.send(E(1));
        clear_func(&mut events);
        assert!(reader.read(&events).next().is_none());

        events.send(E(2));
        events.update();
        events.send(E(3));

        assert!(reader.read(&events).eq([E(2), E(3)].iter()));
    }

    #[test]
    fn test_events_clear_and_read() {
        events_clear_and_read_impl(|events| events.clear());
    }

    #[test]
    fn test_events_drain_and_read() {
        events_clear_and_read_impl(|events| {
            assert!(events.drain().eq(vec![E(0), E(1)].into_iter()));
        });
    }

    #[test]
    fn test_events_extend_impl() {
        let mut events = Events::<TestEvent>::default();
        let mut reader = events.get_reader();

        events.extend(vec![TestEvent { i: 0 }, TestEvent { i: 1 }]);
        assert!(reader
            .read(&events)
            .eq([TestEvent { i: 0 }, TestEvent { i: 1 }].iter()));
    }

    #[test]
    fn test_events_empty() {
        let mut events = Events::<TestEvent>::default();
        assert!(events.is_empty());

        events.send(TestEvent { i: 0 });
        assert!(!events.is_empty());

        events.update();
        assert!(!events.is_empty());

        // events are only empty after the second call to update
        // due to double buffering.
        events.update();
        assert!(events.is_empty());
    }

    #[test]
    fn test_event_reader_len_empty() {
        let events = Events::<TestEvent>::default();
        assert_eq!(events.get_reader().len(&events), 0);
        assert!(events.get_reader().is_empty(&events));
    }

    #[test]
    fn test_event_reader_len_filled() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        assert_eq!(events.get_reader().len(&events), 1);
        assert!(!events.get_reader().is_empty(&events));
    }

    #[test]
    fn test_event_iter_len_updated() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        events.send(TestEvent { i: 1 });
        events.send(TestEvent { i: 2 });
        let mut reader = events.get_reader();
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
    fn test_event_reader_len_current() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        let reader = events.get_reader_current();
        dbg!(&reader);
        dbg!(&events);
        assert!(reader.is_empty(&events));
        events.send(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 1);
        assert!(!reader.is_empty(&events));
    }

    #[test]
    fn test_event_reader_len_update() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        events.send(TestEvent { i: 0 });
        let reader = events.get_reader();
        assert_eq!(reader.len(&events), 2);
        events.update();
        events.send(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 3);
        events.update();
        assert_eq!(reader.len(&events), 1);
        events.update();
        assert!(reader.is_empty(&events));
    }

    #[test]
    fn test_event_reader_clear() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        world.insert_resource(events);

        let mut reader = IntoSystem::into_system(|mut events: EventReader<TestEvent>| -> bool {
            if !events.is_empty() {
                events.clear();
                false
            } else {
                true
            }
        });
        reader.initialize(&mut world);

        let is_empty = reader.run((), &mut world);
        assert!(!is_empty, "EventReader should not be empty");
        let is_empty = reader.run((), &mut world);
        assert!(is_empty, "EventReader should be empty");
    }

    #[test]
    fn test_update_drain() {
        let mut events = Events::<TestEvent>::default();
        let mut reader = events.get_reader();

        events.send(TestEvent { i: 0 });
        events.send(TestEvent { i: 1 });
        assert_eq!(reader.read(&events).count(), 2);

        let mut old_events = Vec::from_iter(events.update_drain());
        assert!(old_events.is_empty());

        events.send(TestEvent { i: 2 });
        assert_eq!(reader.read(&events).count(), 1);

        old_events.extend(events.update_drain());
        assert_eq!(old_events.len(), 2);

        old_events.extend(events.update_drain());
        assert_eq!(
            old_events,
            &[TestEvent { i: 0 }, TestEvent { i: 1 }, TestEvent { i: 2 }]
        );
    }

    #[allow(clippy::iter_nth_zero)]
    #[test]
    fn test_event_iter_nth() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();

        world.send_event(TestEvent { i: 0 });
        world.send_event(TestEvent { i: 1 });
        world.send_event(TestEvent { i: 2 });
        world.send_event(TestEvent { i: 3 });
        world.send_event(TestEvent { i: 4 });

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
    fn test_event_iter_last() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();

        let mut reader =
            IntoSystem::into_system(|mut events: EventReader<TestEvent>| -> Option<TestEvent> {
                events.read().last().copied()
            });
        reader.initialize(&mut world);

        let last = reader.run((), &mut world);
        assert!(last.is_none(), "EventReader should be empty");

        world.send_event(TestEvent { i: 0 });
        let last = reader.run((), &mut world);
        assert_eq!(last, Some(TestEvent { i: 0 }));

        world.send_event(TestEvent { i: 1 });
        world.send_event(TestEvent { i: 2 });
        world.send_event(TestEvent { i: 3 });
        let last = reader.run((), &mut world);
        assert_eq!(last, Some(TestEvent { i: 3 }));

        let last = reader.run((), &mut world);
        assert!(last.is_none(), "EventReader should be empty");
    }

    #[derive(Event, Clone, PartialEq, Debug, Default)]
    struct EmptyTestEvent;

    #[test]
    fn test_firing_empty_event() {
        let mut events = Events::<EmptyTestEvent>::default();
        events.send_default();

        let mut reader = events.get_reader();
        assert_eq!(get_events(&events, &mut reader), vec![EmptyTestEvent]);
    }

    #[test]
    fn ensure_reader_readonly() {
        fn reader_system(_: EventReader<EmptyTestEvent>) {}

        assert_is_read_only_system(reader_system);
    }

    #[test]
    fn test_send_events_ids() {
        let mut events = Events::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        let event_0_id = events.send(event_0);

        assert_eq!(
            events.get_event(event_0_id.id),
            Some((&event_0, event_0_id)),
            "Getting a sent event by ID should return the original event"
        );

        let mut event_ids = events.send_batch([event_1, event_2]);

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

    #[cfg(feature = "multi_threaded")]
    #[test]
    fn test_events_par_iter() {
        use std::{collections::HashSet, sync::mpsc};

        use crate::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();
        for i in 0..100 {
            world.send_event(TestEvent { i });
        }

        let mut schedule = Schedule::default();

        schedule.add_systems(|mut events: EventReader<TestEvent>| {
            let (tx, rx) = mpsc::channel();
            events.par_read().for_each(|event| {
                tx.send(event.i).unwrap();
            });
            drop(tx);

            let observed: HashSet<_> = rx.into_iter().collect();
            assert_eq!(observed, HashSet::from_iter(0..100));
        });
        schedule.run(&mut world);
    }

    // Peak tests
}
