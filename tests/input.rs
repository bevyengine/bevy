
use bevy::{
    asset::AssetPlugin,
    core::CorePlugin,
    core_pipeline::CorePipelinePlugin,
    pbr::PbrPlugin,
    input::InputPlugin,
    prelude::{App, Entity},
    render::{ RenderPlugin},
    window::{WindowPlugin, WindowId, ReceivedCharacter}, utils::{HashSet},
    ecs::event::Events,
    prelude::{EventReader, Component, Commands, KeyCode}, input::{keyboard::KeyboardInput, ButtonState},
};

#[derive(Component)]
struct CharComponent(char);
fn spawn_entity_at_char_event_system(mut commands: Commands, mut char_input_events: EventReader<ReceivedCharacter>) {
    for event in char_input_events.iter() {
        commands.spawn().insert(CharComponent(event.char));
    }
}

#[derive(Component)]
struct KeyComponent(KeyboardInput);
fn spawn_entity_at_keyboard_event_system(mut commands: Commands, mut char_input_events: EventReader<KeyboardInput>) {
    for event in char_input_events.iter() {
        commands.spawn().insert(KeyComponent(event.clone()));
    }
}

macro_rules! get_entities {
    ($app: ident, $query: tt) => {
        {
            let world = &mut $app.world;
            let mut query = world.query::<$query>();
            query.iter(world).collect::<Vec<_>>()
        }
    };
}

macro_rules! send_event {
    ($app: ident, $t: ty, $event: expr) => {
        let world = &mut $app.world;
        let mut sender = world.get_resource_mut::<Events<$t>>().unwrap();
        sender.send($event);
    };
}

#[test]
fn test_input_received_character_single_button() {
    let mut app = App::new();

    app.add_plugin(CorePlugin::default());
    app.add_plugin(WindowPlugin::default());
    app.add_plugin(AssetPlugin::default());
    app.add_plugin(RenderPlugin::default());
    app.add_plugin(CorePipelinePlugin::default());
    app.add_plugin(PbrPlugin::default());

    app.add_system(spawn_entity_at_char_event_system);

    app.update();

    let entities: Vec<_> = get_entities!(app, Entity);
    assert_eq!(entities.len(), 0);

    send_event!(app, ReceivedCharacter, ReceivedCharacter {
        id: WindowId::primary(),
        char: 'a',
    });

    app.update();

    let chars: HashSet<_> = get_entities!(app, (Entity, &CharComponent))
        .into_iter()
        .map(|(_, c)| c.0)
        .collect();
    assert_eq!(chars, HashSet::<_>::from_iter(['a'].into_iter()));

    send_event!(app, ReceivedCharacter, ReceivedCharacter {
        id: WindowId::primary(),
        char: 'B',
    });

    app.update();

    let chars: HashSet<_> = get_entities!(app, (Entity, &CharComponent))
        .into_iter()
        .map(|(_, c)| c.0)
        .collect();
    assert_eq!(chars, HashSet::<_>::from_iter(['a', 'B'].into_iter()));
}

#[test]
fn test_input_received_character_multiple_buttons_at_once() {
    let mut app = App::new();

    app.add_plugin(CorePlugin::default());
    app.add_plugin(WindowPlugin::default());
    app.add_plugin(AssetPlugin::default());
    app.add_plugin(RenderPlugin::default());
    app.add_plugin(CorePipelinePlugin::default());
    app.add_plugin(PbrPlugin::default());

    app.add_system(spawn_entity_at_char_event_system);

    app.update();

    let entities: Vec<_> = get_entities!(app, Entity);
    assert_eq!(entities.len(), 0);

    send_event!(app, ReceivedCharacter, ReceivedCharacter {
        id: WindowId::primary(),
        char: 'a',
    });
    send_event!(app, ReceivedCharacter, ReceivedCharacter {
        id: WindowId::primary(),
        char: 'B',
    });

    app.update();

    let chars: HashSet<_> = get_entities!(app, (Entity, &CharComponent))
        .into_iter()
        .map(|(_, c)| c.0)
        .collect();
    assert_eq!(chars, HashSet::<_>::from_iter(['a', 'B'].into_iter()));
}

#[test]
fn test_input_received_keyboard_single_button() {
    let mut app = App::new();

    app.add_plugin(CorePlugin::default());
    app.add_plugin(WindowPlugin::default());
    app.add_plugin(AssetPlugin::default());
    app.add_plugin(RenderPlugin::default());
    app.add_plugin(CorePipelinePlugin::default());
    app.add_plugin(PbrPlugin::default());
    app.add_plugin(InputPlugin::default());

    app.add_system(spawn_entity_at_keyboard_event_system);

    app.update();

    let entities: Vec<_> = get_entities!(app, Entity);
    assert_eq!(entities.len(), 0);

    send_event!(app, KeyboardInput, KeyboardInput {
        key_code: Some(KeyCode::A),
        scan_code: 0,
        state: ButtonState::Pressed,
    });

    app.update();

    let entities = get_entities!(app, (Entity, &KeyComponent));
    let keys: HashSet<_> = entities
        .into_iter()
        .map(|(_, c)| (c.0.key_code, c.0.state))
        .collect();
    assert_eq!(keys, HashSet::<_>::from_iter([(Some(KeyCode::A), ButtonState::Pressed)].into_iter()));
}

#[test]
fn test_input_received_keyboard_multiple_button_at_once() {
    let mut app = App::new();

    app.add_plugin(CorePlugin::default());
    app.add_plugin(WindowPlugin::default());
    app.add_plugin(AssetPlugin::default());
    app.add_plugin(RenderPlugin::default());
    app.add_plugin(CorePipelinePlugin::default());
    app.add_plugin(PbrPlugin::default());
    app.add_plugin(InputPlugin::default());

    app.add_system(spawn_entity_at_keyboard_event_system);

    app.update();

    let entities: Vec<_> = get_entities!(app, Entity);
    assert_eq!(entities.len(), 0);

    send_event!(app, KeyboardInput, KeyboardInput {
        key_code: Some(KeyCode::A),
        scan_code: 0,
        state: ButtonState::Pressed,
    });
    send_event!(app, KeyboardInput, KeyboardInput {
        key_code: Some(KeyCode::B),
        scan_code: 0,
        state: ButtonState::Released,
    });

    app.update();

    let entities = get_entities!(app, (Entity, &KeyComponent));
    let keys: HashSet<_> = entities
        .into_iter()
        .map(|(_, c)| (c.0.key_code, c.0.state))
        .collect();
    assert_eq!(keys, HashSet::<_>::from_iter([(Some(KeyCode::A), ButtonState::Pressed), (Some(KeyCode::B), ButtonState::Released)].into_iter()));
}


#[test]
fn test_input_event_should_be_handler_just_once() {
    let mut app = App::new();

    app.add_plugin(CorePlugin::default());
    app.add_plugin(WindowPlugin::default());
    app.add_plugin(AssetPlugin::default());
    app.add_plugin(RenderPlugin::default());
    app.add_plugin(CorePipelinePlugin::default());
    app.add_plugin(PbrPlugin::default());
    app.add_plugin(InputPlugin::default());

    app.add_system(spawn_entity_at_keyboard_event_system);
    app.add_system(spawn_entity_at_char_event_system);

    app.update();
    
    let entities: Vec<_> = get_entities!(app, Entity);
    assert_eq!(entities.len(), 0);
    
    send_event!(app, KeyboardInput, KeyboardInput {
        key_code: Some(KeyCode::A),
        scan_code: 0,
        state: ButtonState::Pressed,
    });
    send_event!(app, ReceivedCharacter, ReceivedCharacter {
        id: WindowId::primary(),
        char: 'B',
    });
    
    app.update();

    let entities: Vec<_> = get_entities!(app, Entity);
    assert_eq!(entities.len(), 2);

    (0..10).for_each(|_| app.update());

}