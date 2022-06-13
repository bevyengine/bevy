use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use bevy::{
    ecs::event::{Event, Events},
    input::InputPlugin,
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::App,
    prelude::{EventReader, KeyCode},
    window::{ReceivedCharacter, WindowId, WindowPlugin},
};

#[test]
fn test_input_received_character() {
    let inputs = Arc::new(Mutex::new(VecDeque::from(['a', 'B'])));
    let inputs2 = inputs.clone();
    let system = move |mut chat_input_events: EventReader<ReceivedCharacter>| {
        for event in chat_input_events.iter() {
            assert_eq!(
                event.char,
                inputs2.lock().unwrap().pop_front().expect("foo")
            );
        }
    };

    let mut app = App::new();
    app.add_plugin(WindowPlugin::default()).add_system(system);

    // If no buttons are pressed, the iterator inside the system function is empty
    app.update();
    assert_eq!(inputs.lock().unwrap().len(), 2);

    send_event(
        &mut app,
        ReceivedCharacter {
            id: WindowId::primary(),
            char: 'a',
        },
    );

    // If a button is clicked, the iterator inside the system function returns one item
    app.update();
    assert_eq!(inputs.lock().unwrap().len(), 1);

    // If no buttons are pressed, the past event is forgot
    // And in this iteration no events are received
    app.update();
    assert_eq!(inputs.lock().unwrap().len(), 1);

    send_event(
        &mut app,
        ReceivedCharacter {
            id: WindowId::primary(),
            char: 'B',
        },
    );

    // Another button is clicked, the vec is empty
    app.update();
    assert!(inputs.lock().unwrap().is_empty());
}

#[test]
fn test_input_keyboard_input() {
    let inputs = Arc::new(Mutex::new(VecDeque::from([
        KeyboardInput {
            key_code: Some(KeyCode::A),
            scan_code: 0,
            state: ButtonState::Pressed,
        },
        KeyboardInput {
            key_code: Some(KeyCode::B),
            scan_code: 0,
            state: ButtonState::Released,
        },
    ])));
    let inputs2 = inputs.clone();
    let system = move |mut chat_input_events: EventReader<KeyboardInput>| {
        for event in chat_input_events.iter() {
            assert_eq!(
                event.clone(),
                inputs2.lock().unwrap().pop_front().expect("foo")
            );
        }
    };

    let mut app = App::new();
    app.add_plugin(InputPlugin::default()).add_system(system);

    // If no buttons are pressed, the iterator inside the system function is empty
    app.update();
    assert_eq!(inputs.lock().unwrap().len(), 2);

    send_event(
        &mut app,
        KeyboardInput {
            key_code: Some(KeyCode::A),
            scan_code: 0,
            state: ButtonState::Pressed,
        },
    );

    // If a button is clicked, the iterator inside the system function returns one item
    app.update();
    assert_eq!(inputs.lock().unwrap().len(), 1);

    // If no buttons are pressed, the past event is forgot
    // And in this iteration no events are received
    app.update();
    assert_eq!(inputs.lock().unwrap().len(), 1);

    send_event(
        &mut app,
        KeyboardInput {
            key_code: Some(KeyCode::B),
            scan_code: 0,
            state: ButtonState::Released,
        },
    );

    // Another button is clicked, the vec is empty
    app.update();
    assert!(inputs.lock().unwrap().is_empty());
}

#[test]
fn test_input_multiple_received_char_and_keyboard_input_are_independent() {
    let expected_received_chars = Arc::new(Mutex::new(VecDeque::from(['a', 'B'])));
    let expected_keyboard_inputs = Arc::new(Mutex::new(VecDeque::from([
        KeyboardInput {
            key_code: Some(KeyCode::A),
            scan_code: 0,
            state: ButtonState::Pressed,
        },
        KeyboardInput {
            key_code: Some(KeyCode::B),
            scan_code: 0,
            state: ButtonState::Released,
        },
    ])));
    let expected_received_chars2 = expected_received_chars.clone();
    let expected_keyboard_inputs2 = expected_keyboard_inputs.clone();
    let keyboard_system = move |mut chat_input_events: EventReader<KeyboardInput>| {
        for event in chat_input_events.iter() {
            assert_eq!(
                event.clone(),
                expected_keyboard_inputs2
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("foo")
            );
        }
    };
    let received_char_system = move |mut chat_input_events: EventReader<ReceivedCharacter>| {
        for event in chat_input_events.iter() {
            assert_eq!(
                event.char,
                expected_received_chars2
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("foo")
            );
        }
    };

    let mut app = App::new();
    app.add_plugin(InputPlugin::default())
        .add_plugin(WindowPlugin::default())
        .add_system(keyboard_system)
        .add_system(received_char_system);

    send_event(
        &mut app,
        KeyboardInput {
            key_code: Some(KeyCode::A),
            scan_code: 0,
            state: ButtonState::Pressed,
        },
    );
    send_event(
        &mut app,
        KeyboardInput {
            key_code: Some(KeyCode::B),
            scan_code: 0,
            state: ButtonState::Released,
        },
    );
    send_event(
        &mut app,
        ReceivedCharacter {
            id: WindowId::primary(),
            char: 'a',
        },
    );
    send_event(
        &mut app,
        ReceivedCharacter {
            id: WindowId::primary(),
            char: 'B',
        },
    );

    // Multiple events can be sent multiple time per `update`.
    // The iterator inside the system function returns 2 events
    app.update();
    assert_eq!(expected_keyboard_inputs.lock().unwrap().len(), 0);
    assert_eq!(expected_received_chars.lock().unwrap().len(), 0);
}

fn send_event<E: Event>(app: &mut App, event: E) {
    let world = &mut app.world;
    let mut sender = world.get_resource_mut::<Events<E>>().unwrap();
    sender.send(event);
}
