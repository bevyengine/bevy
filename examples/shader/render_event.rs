//! Simple example demonstrating the use of [`App::init_render_event`] to send events from the
//! render world to the main world

use bevy::{
    diagnostic::FrameCount,
    ecs::system::{LocalBuilder, ParamBuilder},
    prelude::*,
    render::{
        render_event::{MainEventWriter, RenderEventApp},
        Extract, Render, RenderApp,
    },
};

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, RenderEventDemoPlugin))
        .run()
}

// We need a plugin to organize all the systems and render node required for this example
struct RenderEventDemoPlugin;
impl Plugin for RenderEventDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_render_event::<FrameRenderedEvent>()
            .add_systems(Update, read_render_event);
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Since `FrameCount` is not present in the render world by default,
        // we need to initialize it here. This is specific to this example,
        // and should be unnecessary for most users.
        render_app.init_resource::<FrameCount>();

        let send_render_event = (
            ParamBuilder,
            ParamBuilder,
            ParamBuilder,
            LocalBuilder(Timer::from_seconds(2.0, TimerMode::Repeating)),
        )
            .build_state(render_app.world_mut())
            .build_system(send_render_event);

        render_app
            .add_systems(ExtractSchedule, extract_frame_count)
            .add_systems(Render, send_render_event);
    }
}

// Since `FrameCount` is not present in the render world by default,
// we need to extract it here. This is specific to this example, and should
// be unnecessary for most users.
fn extract_frame_count(
    main_frame_count: Extract<Res<FrameCount>>,
    mut render_frame_count: ResMut<FrameCount>,
) {
    //since the frame count gets updated before extraction, it'll appear
    //as one greater than it should be.
    render_frame_count.0 = main_frame_count.0 - 1;
}

#[derive(Event)]
struct FrameRenderedEvent(u32);

fn send_render_event(
    frame_count: Res<FrameCount>,
    mut render_events: MainEventWriter<FrameRenderedEvent>,
    time: Res<Time>,
    mut timer: Local<Timer>,
) {
    timer.tick(time.delta());
    if timer.finished() {
        render_events.write(FrameRenderedEvent(frame_count.0));
    }
}

fn read_render_event(
    mut render_events: EventReader<FrameRenderedEvent>,
    frame_count: Res<FrameCount>,
) {
    for render_event in render_events.read() {
        println!(
            "Render event from frame {} received in frame {}",
            render_event.0, frame_count.0
        );
    }
}
