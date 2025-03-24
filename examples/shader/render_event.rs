//! Simple example demonstrating the use of [`App::init_render_event`] to send events from the
//! render world to the main world

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        Render, RenderApp,
    },
};
use bevy_ecs::system::{LocalBuilder, ParamBuilder};
use bevy_render::render_event::{MainEventWriter, RenderEventApp};

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, RenderEventDemoPlugin))
        .run()
}

// We need a plugin to organize all the systems and render node required for this example
struct RenderEventDemoPlugin;
impl Plugin for RenderEventDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<FrameIndex>::default())
            .init_resource::<FrameIndex>()
            .add_render_event::<FrameRenderedEvent>()
            .add_systems(Update, (increment_frame_index, read_render_event));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<FrameIndex>();

        let send_render_event = (
            ParamBuilder,
            ParamBuilder,
            ParamBuilder,
            LocalBuilder(Timer::from_seconds(2.0, TimerMode::Repeating)),
        )
            .build_state(render_app.world_mut())
            .build_system(send_render_event);

        render_app.add_systems(Render, send_render_event);
    }
}

#[derive(Resource, Default, Clone, Copy, ExtractResource)]
struct FrameIndex(u32);

#[derive(Event)]
struct FrameRenderedEvent(u32);

fn increment_frame_index(mut frame_index: ResMut<FrameIndex>) {
    frame_index.0 += 1;
}

fn send_render_event(
    frame_index: Res<FrameIndex>,
    mut render_events: MainEventWriter<FrameRenderedEvent>,
    time: Res<Time>,
    mut timer: Local<Timer>,
) {
    timer.tick(time.delta());
    if timer.finished() {
        render_events.write(FrameRenderedEvent(frame_index.0));
    }
}

fn read_render_event(
    mut render_events: EventReader<FrameRenderedEvent>,
    frame_index: Res<FrameIndex>,
) {
    for render_event in render_events.read() {
        println!(
            "Render event from frame {} received in frame {}",
            render_event.0, frame_index.0
        );
    }
}
