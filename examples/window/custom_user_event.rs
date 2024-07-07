//! Shows how to create a custom event that can be handled by `winit`'s event loop.

use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WakeUp, WinitPlugin};
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
                // Only one event type can be handled at once
                // so we must disable the default event type
                .disable::<WinitPlugin<WakeUp>>()
                .add(winit_plugin),
        )
        .add_systems(
            Startup,
            (
                setup,
                #[cfg(target_arch = "wasm32")]
                wasm::expose_event_loop_proxy,
                #[cfg(target_arch = "wasm32")]
                wasm::setup_js_closure,
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
    event_loop_proxy: Res<EventLoopProxyWrapper<CustomEvent>>,
) {
    if input.just_pressed(KeyCode::Space) {
        let _ = event_loop_proxy.send_event(CustomEvent::WakeUp);
    }

    // This simulates sending a custom event through an external thread.
    #[cfg(not(target_arch = "wasm32"))]
    if input.just_pressed(KeyCode::KeyE) {
        let event_loop_proxy = event_loop_proxy.clone();
        let handler = std::thread::spawn(move || {
            let _ = event_loop_proxy.clone().send_event(CustomEvent::Key('e'));
        });

        handler.join().unwrap();
    }
}

fn handle_event(mut events: EventReader<CustomEvent>) {
    for evt in events.read() {
        info!("Received event: {evt:?}");
    }
}

/// Since the [`EventLoopProxy`] can be exposed to the javascript environment, it can
/// be used to send events inside the loop, to be handled by a system or simply to wake up
/// the loop if that's currently waiting for a timeout or a user event.
#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm {
    use super::*;
    use bevy::winit::EventLoopProxy;
    use std::sync::OnceLock;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::KeyboardEvent;

    static EVENT_LOOP_PROXY: OnceLock<EventLoopProxy<CustomEvent>> = OnceLock::new();

    pub(crate) fn expose_event_loop_proxy(
        event_loop_proxy: NonSend<EventLoopProxyWrapper<CustomEvent>>,
    ) {
        EVENT_LOOP_PROXY.set((*event_loop_proxy).clone()).unwrap();
    }

    pub(crate) fn setup_js_closure() {
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
        if let Some(proxy) = EVENT_LOOP_PROXY.get() {
            proxy
                .send_event(CustomEvent::Key(ch))
                .map_err(|_| "Failed to send event".to_string())
        } else {
            Err("Event loop proxy not found".to_string())
        }
    }
}
