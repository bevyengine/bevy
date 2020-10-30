#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
};

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");
    }

    App::build()
        .add_resource(WindowDescriptor {
            width: 300,
            height: 300,
            ..Default::default()
        })
        .add_default_plugins()
        // One time greet
        .add_startup_system(hello_wasm_system.system())
        // Track ticks (sanity check, whether game loop is running)
        .add_system(counter.system())
        // Track input events
        .init_resource::<TrackInputState>()
        .add_system(track_input_events.system())
        .run();
}

fn hello_wasm_system() {
    log::info!("hello wasm");
}

fn counter(mut state: Local<CounterState>, time: Res<Time>) {
    if state.count % 60 == 0 {
        log::info!(
            "tick {} @ {:?} [Δ{}]",
            state.count,
            time.time_since_startup(),
            time.delta_seconds
        );
    }
    state.count += 1;
}

#[derive(Default)]
struct CounterState {
    count: u32,
}

#[derive(Default)]
struct TrackInputState {
    keys: EventReader<KeyboardInput>,
    cursor: EventReader<CursorMoved>,
    motion: EventReader<MouseMotion>,
    mousebtn: EventReader<MouseButtonInput>,
    scroll: EventReader<MouseWheel>,
}

fn track_input_events(
    mut state: ResMut<TrackInputState>,
    ev_keys: Res<Events<KeyboardInput>>,
    ev_cursor: Res<Events<CursorMoved>>,
    ev_motion: Res<Events<MouseMotion>>,
    ev_mousebtn: Res<Events<MouseButtonInput>>,
    ev_scroll: Res<Events<MouseWheel>>,
) {
    // Keyboard input
    for ev in state.keys.iter(&ev_keys) {
        if ev.state.is_pressed() {
            log::info!("Just pressed key: {:?}", ev.key_code);
        } else {
            log::info!("Just released key: {:?}", ev.key_code);
        }
    }

    // Absolute cursor position (in window coordinates)
    for ev in state.cursor.iter(&ev_cursor) {
        log::info!("Cursor at: {}", ev.position);
    }

    // Relative mouse motion
    for ev in state.motion.iter(&ev_motion) {
        log::info!("Mouse moved {} pixels", ev.delta);
    }

    // Mouse buttons
    for ev in state.mousebtn.iter(&ev_mousebtn) {
        if ev.state.is_pressed() {
            log::info!("Just pressed mouse button: {:?}", ev.button);
        } else {
            log::info!("Just released mouse button: {:?}", ev.button);
        }
    }

    // scrolling (mouse wheel, touchpad, etc.)
    for ev in state.scroll.iter(&ev_scroll) {
        log::info!(
            "Scrolled vertically by {} and horizontally by {}.",
            ev.y,
            ev.x
        );
    }
}
