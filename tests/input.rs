use bevy::{
    ecs::{
        event::{Event, Events},
        query::{Fetch, WorldQuery, WorldQueryGats},
    },
    input::InputPlugin,
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::{App, Entity},
    prelude::{Commands, Component, EventReader, KeyCode},
    utils::HashSet,
    window::{ReceivedCharacter, WindowId, WindowPlugin},
};

#[derive(Component)]
struct CharComponent(char);
fn spawn_entity_at_char_event_system(
    mut commands: Commands,
    mut char_input_events: EventReader<ReceivedCharacter>,
) {
    for event in char_input_events.iter() {
        commands.spawn().insert(CharComponent(event.char));
    }
}

#[derive(Component)]
struct KeyComponent(KeyboardInput);
fn spawn_entity_at_keyboard_event_system(
    mut commands: Commands,
    mut char_input_events: EventReader<KeyboardInput>,
) {
    for event in char_input_events.iter() {
        commands.spawn().insert(KeyComponent(event.clone()));
    }
}

#[test]
fn test_input_received_character_single_button() {
    let mut app = App::new();
    app.add_plugin(WindowPlugin::default())
        .add_system(spawn_entity_at_char_event_system);

    app.update();

    let entities: Vec<Entity> = get_entities(&mut app);
    assert_eq!(entities.len(), 0);

    send_event(
        &mut app,
        ReceivedCharacter {
            id: WindowId::primary(),
            char: 'a',
        },
    );

    app.update();

    let chars: HashSet<_> = get_entities::<(Entity, &CharComponent)>(&mut app)
        .into_iter()
        .map(|(_, c)| c.0)
        .collect();
    assert_eq!(chars, HashSet::<_>::from_iter(['a'].into_iter()));

    send_event(
        &mut app,
        ReceivedCharacter {
            id: WindowId::primary(),
            char: 'B',
        },
    );

    app.update();

    let entities: Vec<(Entity, &CharComponent)> = get_entities(&mut app);
    let chars: HashSet<_> = entities.into_iter().map(|(_, c)| c.0).collect();
    assert_eq!(chars, HashSet::<_>::from_iter(['a', 'B'].into_iter()));
}

#[test]
fn test_input_received_character_multiple_buttons_at_once() {
    let mut app = App::new();
    app.add_plugin(WindowPlugin::default())
        .add_system(spawn_entity_at_char_event_system);

    app.update();

    let entities: Vec<Entity> = get_entities(&mut app);
    assert_eq!(entities.len(), 0);

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

    app.update();

    let entities: Vec<(Entity, &CharComponent)> = get_entities(&mut app);
    let chars: HashSet<_> = entities.into_iter().map(|(_, c)| c.0).collect();
    assert_eq!(chars, HashSet::<_>::from_iter(['a', 'B'].into_iter()));
}

#[test]
fn test_input_received_keyboard_single_button() {
    let mut app = App::new();

    app.add_plugin(InputPlugin::default())
        .add_system(spawn_entity_at_keyboard_event_system);

    app.update();

    let entities: Vec<Entity> = get_entities(&mut app);
    assert_eq!(entities.len(), 0);

    send_event(
        &mut app,
        KeyboardInput {
            key_code: Some(KeyCode::A),
            scan_code: 0,
            state: ButtonState::Pressed,
        },
    );

    app.update();

    let entities: Vec<(Entity, &KeyComponent)> = get_entities(&mut app);
    let keys: HashSet<_> = entities
        .into_iter()
        .map(|(_, c)| (c.0.key_code, c.0.state))
        .collect();
    assert_eq!(
        keys,
        HashSet::<_>::from_iter([(Some(KeyCode::A), ButtonState::Pressed)].into_iter())
    );
}

#[test]
fn test_input_received_keyboard_multiple_button_at_once() {
    let mut app = App::new();

    app.add_plugin(InputPlugin::default())
        .add_system(spawn_entity_at_keyboard_event_system);

    app.update();

    let entities: Vec<Entity> = get_entities(&mut app);
    assert_eq!(entities.len(), 0);

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

    app.update();

    let entities: Vec<(Entity, &KeyComponent)> = get_entities(&mut app);
    let keys: HashSet<_> = entities
        .into_iter()
        .map(|(_, c)| (c.0.key_code, c.0.state))
        .collect();
    assert_eq!(
        keys,
        HashSet::<_>::from_iter(
            [
                (Some(KeyCode::A), ButtonState::Pressed),
                (Some(KeyCode::B), ButtonState::Released)
            ]
            .into_iter()
        )
    );
}

#[test]
fn test_input_event_should_be_handler_just_once() {
    let mut app = App::new();
    app.add_plugin(WindowPlugin::default())
        .add_plugin(InputPlugin::default())
        .add_system(spawn_entity_at_keyboard_event_system)
        .add_system(spawn_entity_at_char_event_system);

    app.update();

    let entities: Vec<Entity> = get_entities(&mut app);
    assert_eq!(entities.len(), 0);

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
        ReceivedCharacter {
            id: WindowId::primary(),
            char: 'B',
        },
    );

    app.update();

    let entities: Vec<Entity> = get_entities(&mut app);
    assert_eq!(entities.len(), 2);

    (0..10).for_each(|_| app.update());

    let entities: Vec<Entity> = get_entities(&mut app);
    assert_eq!(entities.len(), 2);
}

fn get_entities<'a, T: WorldQuery>(app: &'a mut App) -> Vec<T>
where
    Vec<T>: FromIterator<<<T as WorldQueryGats<'a>>::ReadOnlyFetch as Fetch<'a>>::Item>,
{
    let world = &mut app.world;
    let mut query = world.query::<T>();
    let iter = query.iter(world);

    iter.collect()
}

fn send_event<E: Event>(app: &mut App, event: E) {
    let world = &mut app.world;
    let mut sender = world.get_resource_mut::<Events<E>>().unwrap();
    sender.send(event);
}
