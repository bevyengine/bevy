use crate::{WindowCloseRequested, WindowId};
use bevy_app::{AppExit, EventReader, Events};
use legion::prelude::*;

pub fn exit_on_window_close_system(window_id: Option<WindowId>) -> Box<dyn Schedulable> {
    let mut window_close_requested_event_reader = EventReader::<WindowCloseRequested>::default();
    SystemBuilder::new("exit_on_window_close")
        .read_resource::<Events<WindowCloseRequested>>()
        .write_resource::<Events<AppExit>>()
        .build(
            move |_, _, (ref window_close_requested_events, ref mut app_exit_events), _| {
                for window_close_requested_event in
                    window_close_requested_event_reader.iter(window_close_requested_events)
                {
                    match window_id.as_ref() {
                        Some(window_id) => {
                            if *window_id == window_close_requested_event.id {
                                app_exit_events.send(AppExit);
                            }
                        }
                        None => {
                            if window_close_requested_event.is_primary {
                                app_exit_events.send(AppExit);
                            }
                        }
                    }
                }
            },
        )
}
