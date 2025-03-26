use crate::{ReflectComponent, ReflectDefault};
use bevy_app::{Plugin, PreUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    resource::Resource,
    system::{Query, ResMut},
};
use bevy_picking::events::*;
use bevy_reflect::{FromReflect, GetTypeRegistration, Reflect, Typed};
use std::{collections::VecDeque, fmt::Debug, marker::PhantomData};

/// A widget can subscribe to an event type with this component
#[derive(Component, Debug, Clone, PartialEq, Reflect, Deref, DerefMut)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct EventsReactor<T: Event>(pub(crate) VecDeque<T>);

impl<T: Event + 'static> Default for EventsReactor<T> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

/// A resource acts as a single sender per event type
#[derive(Resource)]
pub struct EventsDispatch<T: Event + 'static> {
    queue: VecDeque<T>,
}

impl<T: Event + 'static> Default for EventsDispatch<T> {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl<T: Event + 'static> EventsDispatch<T> {
    pub fn send(&mut self, event: T) {
        self.queue.push_back(event);
    }
}

/// Marker trait for a user event type
pub trait Event
where
    Self: Clone + PartialEq + Debug + Send + Sync + 'static,
{
}

impl<T> Event for T where T: Clone + PartialEq + Debug + Send + Sync + 'static {}

/// A generic events relay plugin
pub struct EventsPlugin<T: Event> {
    _phantom: PhantomData<T>,
}

impl<T: Event> Default for EventsPlugin<T> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> Plugin for EventsPlugin<T>
where
    T: Event + Typed + GetTypeRegistration + FromReflect + 'static,
{
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<EventsDispatch<T>>();
        app.register_type::<EventsReactor<T>>();
        app.register_type::<T>();
        app.add_systems(PreUpdate, relay_events::<T>);
    }
}

/// Take queued events from a dispatch and insert it into every reactor for the same type
fn relay_events<T: Event + 'static>(
    mut dispatch: ResMut<EventsDispatch<T>>,
    mut q: Query<&mut EventsReactor<T>>,
) {
    for ev in &mut dispatch.queue {
        for mut reactor in q.iter_mut() {
            reactor.0.push_back(ev.clone());
        }
    }
}

/// A simple map of `bevy_picking` input events to `bevy_ui::widget` interaction events
#[derive(Clone, PartialEq, Debug, Reflect, Component)]
#[reflect(Debug, Clone)]
pub enum PickingEvent {
    Over(Pointer<Over>),
    Out(Pointer<Out>),
    Click(Pointer<Click>),
    Move(Pointer<Move>),
    DragStart(Pointer<DragStart>),
    Drag(Pointer<Drag>),
    DragEnd(Pointer<DragEnd>),
    DragEnter(Pointer<DragEnter>),
    DragOver(Pointer<DragOver>),
    DragLeave(Pointer<DragLeave>),
    DragDrop(Pointer<DragDrop>),
    DragEntry(Pointer<DragEntry>),
    Scroll(Pointer<Scroll>),
}
