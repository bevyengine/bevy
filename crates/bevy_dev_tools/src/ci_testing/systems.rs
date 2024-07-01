use super::config::*;
use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_render::view::screenshot::ScreenshotManager;
use bevy_utils::tracing::{debug, info, warn};
use bevy_window::PrimaryWindow;

pub(crate) fn send_events(world: &mut World, mut current_frame: Local<u32>) {
    let mut config = world.resource_mut::<CiTestingConfig>();

    // Take all events for the current frame, leaving all the remaining alone.
    let events = std::mem::take(&mut config.events);
    let (to_run, remaining): (Vec<_>, _) = events
        .into_iter()
        .partition(|event| event.0 == *current_frame);
    config.events = remaining;

    for CiTestingEventOnFrame(_, event) in to_run {
        debug!("Handling event: {:?}", event);
        match event {
            CiTestingEvent::AppExit => {
                world.send_event(AppExit::Success);
                info!("Exiting after {} frames. Test successful!", *current_frame);
            }
            CiTestingEvent::Screenshot => {
                let mut primary_window_query =
                    world.query_filtered::<Entity, With<PrimaryWindow>>();
                let Ok(main_window) = primary_window_query.get_single(world) else {
                    warn!("Requesting screenshot, but PrimaryWindow is not available");
                    continue;
                };
                let Some(mut screenshot_manager) = world.get_resource_mut::<ScreenshotManager>()
                else {
                    warn!("Requesting screenshot, but ScreenshotManager is not available");
                    continue;
                };
                let path = format!("./screenshot-{}.png", *current_frame);
                screenshot_manager
                    .save_screenshot_to_disk(main_window, path)
                    .unwrap();
                info!("Took a screenshot at frame {}.", *current_frame);
            }
            // Custom events are forwarded to the world.
            CiTestingEvent::Custom(event_string) => {
                world.send_event(CiTestingCustomEvent(event_string));
            }
        }
    }

    *current_frame += 1;
}
