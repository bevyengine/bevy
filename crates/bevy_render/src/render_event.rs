use core::marker::PhantomData;

use async_channel::{Receiver, Sender};
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    change_detection::MaybeLocation,
    event::{Event, Events},
    resource::Resource,
    system::{Res, ResMut, SystemParam},
};

use crate::RenderApp;

pub trait RenderEventApp {
    fn add_render_event<E: Event>(&mut self) -> &mut Self;
}

impl RenderEventApp for App {
    fn add_render_event<E: Event>(&mut self) -> &mut Self {
        self.add_plugins(RenderEventPlugin::<E>::default())
    }
}

struct RenderEventPlugin<E: Event>(PhantomData<E>);

impl<E: Event> Default for RenderEventPlugin<E> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<E: Event> Plugin for RenderEventPlugin<E> {
    fn build(&self, app: &mut App) {
        app.add_event::<E>()
            .add_systems(PreUpdate, relay_render_events::<E>);
    }

    fn finish(&self, app: &mut App) {
        let (sender, receiver) = async_channel::unbounded::<(E, MaybeLocation)>();

        app.insert_resource(RenderEventReceiver(receiver));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.insert_resource(RenderEventSender(sender));
    }
}

#[derive(Resource)]
struct RenderEventReceiver<E: Event>(Receiver<(E, MaybeLocation)>);
#[derive(Resource)]
struct RenderEventSender<E: Event>(Sender<(E, MaybeLocation)>);

fn relay_render_events<E: Event>(
    mut events: ResMut<Events<E>>,
    receiver: Res<RenderEventReceiver<E>>,
) {
    while let Ok((event, caller)) = receiver.0.try_recv() {
        events.send_with_caller(event, caller);
    }
}

/// An event writer for sending events from the render world to the main world.
///
/// Internally, this struct writes to a channel, the contents of which are relayed
/// to the main world's [`Events`] stream during [`PreUpdate`]. Note that because
/// the render world is pipelined, the events may not arrive before the next frame begins.
#[derive(SystemParam)]
pub struct MainEventWriter<'w, E: Event> {
    sender: ResMut<'w, RenderEventSender<E>>,
}

const ERR_MSG: &str = "A render events channel has been closed. This is illegal";

impl<'w, E: Event> MainEventWriter<'w, E> {
    /// Writes an `event`, which can later be read by [`EventReader`](super::EventReader)s
    /// in the main world.
    ///
    /// See [`Events`] for details.
    #[doc(alias = "send")]
    #[track_caller]
    pub fn write(&mut self, event: E) {
        self.sender
            .0
            .try_send((event, MaybeLocation::caller()))
            .expect(ERR_MSG);
    }

    /// Sends a list of `events` all at once, which can later be read
    /// by [`EventReader`](super::EventReader)s in the main world.
    /// This is more efficient than sending each event individually.
    ///
    /// See [`Events`] for details.
    #[doc(alias = "send_batch")]
    #[track_caller]
    pub fn write_batch(&mut self, events: impl IntoIterator<Item = E>) {
        events.into_iter().for_each(|event| {
            self.sender
                .0
                .try_send((event, MaybeLocation::caller()))
                .expect(ERR_MSG);
        });
    }

    /// Writes the default value of the event. Useful when the event is an empty struct.
    ///
    /// See [`Events`] for details.
    #[doc(alias = "send_default")]
    #[track_caller]
    pub fn write_default(&mut self)
    where
        E: Default,
    {
        self.sender
            .0
            .try_send((Default::default(), MaybeLocation::caller()))
            .expect(ERR_MSG);
    }
}
