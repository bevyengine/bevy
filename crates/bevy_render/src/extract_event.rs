use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
pub use bevy_render_macros::ExtractResource;

/// Describes how an event gets extracted for rendering.
///
/// Therefore the event is transferred from the "main world" into the "render world"
/// in the [`ExtractSchedule`] step.
pub trait ExtractEvent {
    type Source: Event;

    /// Defines how the event is transferred into the "render world".
    fn extract_event(source: &Self::Source) -> Self;
}

/// This plugin extracts events into the "render world".
///
/// The event `E::Source` is automatically added to the app and event `E` is added to the render app.
pub struct ExtractEventPlugin<E: Event + ExtractEvent>(PhantomData<E>);

impl<E: Event + ExtractEvent> Default for ExtractEventPlugin<E> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<E: Event + ExtractEvent> Plugin for ExtractEventPlugin<E> {
    fn build(&self, app: &mut App) {
        app.add_event::<E::Source>();
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_event::<E>();
            render_app.add_systems(ExtractSchedule, extract_event::<E>);
        }
    }
}

/// This system extracts the events of the corresponding [`Event`] type
pub fn extract_event<T: Event + ExtractEvent>(
    mut reader: Local<ManualEventReader<T::Source>>,
    incoming: Extract<Res<Events<T::Source>>>,
    mut outgoing: ResMut<Events<T>>,
) {
    for source in reader.read(&incoming) {
        outgoing.send(T::extract_event(source));
    }
}
