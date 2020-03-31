use crate::prelude::Resources;
use legion::prelude::{Schedulable, SystemBuilder};
use std::marker::PhantomData;

struct EventInstance<T> {
    pub event_count: usize,
    pub event: T,
}

enum State {
    A,
    B,
}

pub struct Event<T>
where
    T: Send + Sync + 'static,
{
    events_a: Vec<EventInstance<T>>,
    events_b: Vec<EventInstance<T>>,
    a_start_event_count: usize,
    b_start_event_count: usize,
    event_count: usize,
    state: State,
}

impl<T> Default for Event<T>
where
    T: Send + Sync + 'static,
{
    fn default() -> Self {
        Event {
            a_start_event_count: 0,
            b_start_event_count: 0,
            event_count: 0,
            events_a: Vec::new(),
            events_b: Vec::new(),
            state: State::A,
        }
    }
}

fn map_event_instance<T>(event_instance: &EventInstance<T>) -> &T
where
    T: Send + Sync + 'static,
{
    &event_instance.event
}

pub struct EventHandle<T> {
    last_event_count: usize,
    _marker: PhantomData<T>,
}

impl<T> Event<T>
where
    T: Send + Sync + 'static,
{
    pub fn send(&mut self, event: T) {
        let event_instance = EventInstance {
            event,
            event_count: self.event_count,
        };

        match self.state {
            State::A => self.events_a.push(event_instance),
            State::B => self.events_b.push(event_instance),
        }

        self.event_count += 1;
    }

    pub fn iter(&self, event_handle: &mut EventHandle<T>) -> impl DoubleEndedIterator<Item = &T> {
        let a_index = self.a_start_event_count - event_handle.last_event_count;
        let b_index = self.b_start_event_count - event_handle.last_event_count;
        event_handle.last_event_count = self.event_count;
        match self.state {
            State::A => self
                .events_b
                .get(b_index..)
                .unwrap_or_else(|| &[])
                .iter()
                .map(map_event_instance)
                .chain(
                    self.events_a
                        .get(a_index..)
                        .unwrap_or_else(|| &[])
                        .iter()
                        .map(map_event_instance),
                ),
            State::B => self
                .events_a
                .get(a_index..)
                .unwrap_or_else(|| &[])
                .iter()
                .map(map_event_instance)
                .chain(
                    self.events_b
                        .get(b_index..)
                        .unwrap_or_else(|| &[])
                        .iter()
                        .map(map_event_instance),
                ),
        }
    }

    pub fn get_handle(&self) -> EventHandle<T> {
        EventHandle {
            last_event_count: 0,
            _marker: PhantomData,
        }
    }

    pub fn update(&mut self) {
        match self.state {
            State::A => {
                self.events_b = Vec::new();
                self.state = State::B;
                self.b_start_event_count = self.event_count;
            }
            State::B => {
                self.events_a = Vec::new();
                self.state = State::A;
                self.a_start_event_count = self.event_count;
            }
        }
    }

    pub fn update_system() -> Box<dyn Schedulable> {
        SystemBuilder::new(format!("EventUpdate::{}", std::any::type_name::<T>()))
            .write_resource::<Self>()
            .build(|_, _, event, _| event.update())
    }
}

pub trait GetEventHandle {
    fn get_event_handle<T>(&self) -> EventHandle<T>
    where
        T: Send + Sync + 'static;
}

impl GetEventHandle for Resources {
    fn get_event_handle<T>(&self) -> EventHandle<T>
    where
        T: Send + Sync + 'static,
    {
        let my_event = self
            .get::<Event<T>>()
            .unwrap_or_else(|| panic!("Event does not exist: {}", std::any::type_name::<T>()));
        my_event.get_handle()
    }
}
