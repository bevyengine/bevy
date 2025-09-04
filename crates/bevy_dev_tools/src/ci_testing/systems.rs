use super::config::*;
use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_render::view::screenshot::{save_to_disk, Screenshot};
use tracing::{debug, info};

pub(crate) fn send_events(world: &mut World, mut current_frame: Local<u32>) {
    let mut config = world.resource_mut::<CiTestingConfig>();

    // Take all events for the current frame, leaving all the remaining alone.
    let events = core::mem::take(&mut config.events);
    let (to_run, remaining): (Vec<_>, _) = events
        .into_iter()
        .partition(|event| event.0 == *current_frame);
    config.events = remaining;

    for CiTestingEventOnFrame(_, event) in to_run {
        debug!("Handling event: {:?}", event);
        match event {
            CiTestingEvent::AppExit => {
                world.write_event(AppExit::Success);
                info!("Exiting after {} frames. Test successful!", *current_frame);
            }
            CiTestingEvent::ScreenshotAndExit => {
                let this_frame = *current_frame;
                world.spawn(Screenshot::primary_window()).observe(
                    move |captured: On<bevy_render::view::screenshot::ScreenshotCaptured>,
                          mut exit_event: EventWriter<AppExit>| {
                        let path = format!("./screenshot-{this_frame}.png");
                        save_to_disk(path)(captured);
                        info!("Exiting. Test successful!");
                        exit_event.write(AppExit::Success);
                    },
                );
                info!("Took a screenshot at frame {}.", *current_frame);
            }
            CiTestingEvent::Screenshot => {
                let path = format!("./screenshot-{}.png", *current_frame);
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
                info!("Took a screenshot at frame {}.", *current_frame);
            }
            CiTestingEvent::NamedScreenshot(name) => {
                let path = format!("./screenshot-{name}.png");
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
                info!(
                    "Took a screenshot at frame {} for {}.",
                    *current_frame, name
                );
            }
            // Custom events are forwarded to the world.
            CiTestingEvent::Custom(event_string) => {
                world.write_event(CiTestingCustomEvent(event_string));
            }
        }
    }

    *current_frame += 1;
}
