//! Shows how to create a custom event that can be handled by the event loop.

use bevy::prelude::*;
use bevy::winit::{EventLoopProxy, WakeUp, WinitPlugin};
use std::fmt::Formatter;

#[derive(Default, Debug, Event)]
enum CustomEvent {
    #[default]
    WakeUp,
    Key(char),
}

impl std::fmt::Display for CustomEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WakeUp => write!(f, "Wake up"),
            Self::Key(ch) => write!(f, "Key: {ch}"),
        }
    }
}

fn main() {
    let winit_plugin = WinitPlugin::<CustomEvent>::default();

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<WinitPlugin<WakeUp>>()
                .add(winit_plugin),
        )
        .add_systems(
            Startup,
            (
                setup,
                #[cfg(target_arch = "wasm32")]
                wasm::expose_event_loop_proxy,
            ),
        )
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
        let _ = event_loop_proxy.send_event(CustomEvent::WakeUp);
    }
    if input.just_pressed(KeyCode::KeyE) {
        let _ = event_loop_proxy.send_event(CustomEvent::Key('e'));
    }
}

fn handle_event(mut events: EventReader<CustomEvent>) {
    for evt in events.read() {
        info!("Received event: {evt:?}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm {
    use std::sync::{Arc, Mutex};

    use bevy::{ecs::system::NonSend, winit::EventLoopProxy};
    use once_cell::sync::Lazy;

    use crate::CustomEvent;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::KeyboardEvent;

    pub static EVENT_LOOP_PROXY: Lazy<Arc<Mutex<Option<EventLoopProxy<CustomEvent>>>>> =
        Lazy::new(|| Arc::new(Mutex::new(None)));

    pub(crate) fn expose_event_loop_proxy(event_loop_proxy: NonSend<EventLoopProxy<CustomEvent>>) {
        *EVENT_LOOP_PROXY.lock().unwrap() = Some((*event_loop_proxy).clone());

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let key = event.key();
            if key == "e" {
                send_custom_event('e').unwrap();
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

        document
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .unwrap();

        closure.forget();
    }

    fn send_custom_event(ch: char) -> Result<(), String> {
        let proxy = EVENT_LOOP_PROXY.lock().unwrap();
        if let Some(proxy) = &*proxy {
            proxy
                .send_event(CustomEvent::Key(ch))
                .map_err(|_| "Failed to send event".to_string())
        } else {
            Err("Event loop proxy not found".to_string())
        }
    }
}
