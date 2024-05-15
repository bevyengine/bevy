//! Shows how to create a custom event that can be handled by the event loop.

use bevy::prelude::*;
use bevy::winit::{EventLoopProxy, WakeUp, WinitPlugin};

#[derive(Default, Event)]
struct CustomEvent {}

fn main() {
    let winit_plugin = WinitPlugin::<CustomEvent>::default();

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<WinitPlugin<WakeUp>>()
                .add(winit_plugin),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, (send_event, handle_event))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn send_event(
    input: Res<ButtonInput<KeyCode>>,
    event_loop_proxy: NonSend<EventLoopProxy<CustomEvent>>,
) {
    if input.just_pressed(KeyCode::Space) {
        let _ = event_loop_proxy.send_event(CustomEvent {});
        info!("Sending custom event through the proxy");
    }
}

fn handle_event(mut events: EventReader<CustomEvent>) {
    for _ in events.read() {
        info!("Received event");
    }
}
