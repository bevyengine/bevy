use bevy::prelude::*;
use bevy::{
    app::{Events, ManualEventReader},
    core::FixedTimestep,
};

/// In this example, we show how to store events of a given type
/// as a component on individual entities rather than in a single resource.
///
/// This pattern allows you to dispatch events directly to the entity that needs to handle them,
/// letting you avoid storing the `Entity` in the event, and prevents your from needing to either
/// repeatedly scan the entire event list for relevant events or look-up the appropriate entity using
/// slow query.get(my_entity) calls that have poor cache-locality.
///
/// By storing the events on particular entities,
/// you can treat each entity as a seperate event-channel,
/// letting you create new events intended for only certain consumers
/// without forcing you to create a new event type to disambiguate.
///
/// This specific example shows a simple input -> action dispatch use case,
/// where this pattern helps to avoid messy rechecking and allows simple merging of multiple event input streams.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // Adding events using .add_event::<T> will cause all resources and components of type T
        // to be automatically cleaned in a double-buffer fashion by inserting an appropriate system
        //
        // You can avoid this behavior and manually clean up your events by simply adding events
        // as vanilla components or resources
        .add_event::<CycleColorAction>()
        .add_event::<AddNumberAction>()
        .add_startup_system(setup.system())
        .add_system(select_entity.system())
        .add_system(
            input_dispatch
                .system()
                .label("input_dispatch")
                .before("action_handling"),
        )
        .add_system(cycle_color.system().label("action_handling"))
        .add_system(add_number.system().label("action_handling"))
        .add_system(scale_selected.system().after("action_handling"))
        .add_system(update_text_color.system().after("action_handling"))
        .add_system(
            move_text
                .system()
                .label("action_handling")
                .with_run_criteria(FixedTimestep::step(TIME_STEP)),
        )
        .run()
}

pub mod setup {
    #[derive(Bundle)]
    struct InteractableBundle {
        #[bundle]
        text_bundle: TextBundle,
        selectable: Selectable,
        rainbow: ColorChoices,
        cycle_color_events: Events<CycleColorAction>,
        move_events: Events<MoveAction>,
        add_number_events: Events<AddNumberAction>,
    }
    
    impl InteractableBundle {
        // FIXME: fix position
        fn new(x: f32, y: f32, font_handle: &Handle<Font>) -> Self {
            InteractableBundle {
                text_bundle: TextBundle {
                    text: Text::with_section(
                        "0",
                        TextStyle {
                            font: font_handle.clone(),
                            font_size: 60.0,
                            color: Color::WHITE,
                        },
                        TextAlignment {
                            vertical: VerticalAlign::Center,
                            horizontal: HorizontalAlign::Center,
                        },
                    ),
                    transform: Transform::from_xyz(x, y, 0.0),
                    ..Default::default()
                },
                selectable: Selectable,
                rainbow: ColorChoices::Red,
                cycle_color_events: Events::<CycleColorAction>::default(),
                move_events: Events<MoveAction>::default(),
                add_number_events: Events::<AddNumberAction>::default(),
            }
        }
    }
    
    fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    
        let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");
        // Spawns the first entity, and grabs the Entity id that is being allocated
        let first_entity = commands
            .spawn_bundle(InteractableBundle::new(-200.0, 0.0, &font_handle))
            .id();
        commands.insert_resource(Selected {
            entity: first_entity,
        });
    
        commands.spawn_bundle(InteractableBundle::new(0.0, 0.0, &font_handle));
        commands.spawn_bundle(InteractableBundle::new(200.0, 0.0, &font_handle));
    }
    
}

pub mod selection {
// Tracks which entity is selected
struct Selected {
    entity: Entity,
}
// Marks entities as selectable
struct Selectable;

enum CycleBehavior {
    Forward,
    Back,
}

/// Cycles through entities appropriately based on input
fn select_entity(
    mut query: Query<Entity, With<Selectable>>,
    mut selected: ResMut<Selected>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    let cycle_behavior: CycleBehavior = if keyboard_input.just_pressed(KeyCode::Tab) {
        if keyboard_input.pressed(KeyCode::LShift) || keyboard_input.pressed(KeyCode::RShift) {
            CycleBehavior::Back
        } else {
            CycleBehavior::Forward
        }
    } else {
        return;
    };

    let mut entities = Vec::<Entity>::new();
    // FIXME: Move to `.for_each` when https://github.com/bevyengine/bevy/issues/753 is resolved
    query.for_each_mut(|entity| entities.push(entity.clone()));

    let current_position = entities.iter().position(|&e| e == selected.entity).unwrap() as isize;

    let new_position = match cycle_behavior {
        // We have to convert to isize for this step to avoid underflows when current_postion == 0
        CycleBehavior::Forward => (current_position + 1).rem_euclid(entities.len() as isize),
        CycleBehavior::Back => (current_position - 1).rem_euclid(entities.len() as isize),
    };

    selected.entity = entities[new_position as usize];
}

fn scale_selected(
    mut query: Query<(Entity, &mut Text), With<Selectable>>,
    selected: Res<Selected>,
) {
    // Only do work when the selection is changed
    if !selected.is_changed() {
        return;
    }

    for (entity, mut text) in query.iter_mut() {
        if entity == selected.entity {
            text.sections[0].style.font_size = 90.0;
        } else {
            text.sections[0].style.font_size = 60.0;
        }
    }
}

}

pub mod input {
/// Dispatches actions to entities based on the input
/// Note that we can store several events at once!
/// Try pressing both "1" and "3" to add 4 to the selected display
fn input_dispatch(
    // You could also access the &Events<T> component directly
    // then send events to that component with `Events::send`
    mut query: Query<
        (
            EventWriter<CycleColorAction>,
            EventWriter<MoveAction>,
            EventWriter<AddNumberAction>,
        ),
        With<Selectable>,
    >,
    selected: Res<Selected>,
    keyboard_input: ResMut<Input<KeyCode>>,
) {
    use KeyCode::*;

    let (mut cycle_actions, mut add_actions) = query.get_mut(selected.entity).unwrap();

    for key_code in keyboard_input.get_just_pressed() {
        match key_code {
            // Color changing
            Space => cycle_actions.send(CycleColorAction),
            // Movement
            Left => move_actions.send(MoveAction {
                transform: Transform::from_xyz(-MOVE_DISTANCE, 0.0, 0.0),
            }),
            Right => move_actions.send(MoveAction {
                transform: Transform::from_xyz(MOVE_DISTANCE, 0.0, 0.0),
            }),
            Down => move_actions.send(MoveAction {
                transform: Transform::from_xyz(0.0, -MOVE_DISTANCE, 0.0),
            }),
            Up => move_actions.send(MoveAction {
                transform: Transform::from_xyz(0.0, MOVE_DISTANCE, 0.0),
            }),
            _ => (),
        }

        // Inputs for sending numbers to be added
        if (key_code as u8) < 10 {
            add_actions.send(AddNumberAction {
                // The keycode for KeyCode::Key1 is 0
                number: key_code as u8 + 1,
            });
        }
    }
}

}

pub mod colors {
    struct CycleColorAction;
    enum ColorChoices {
        Red,
        Blue,
        Violet,
    }
    
    impl Iterator for ColorChoices {
        type Item = Self;
    
        fn next(&mut self) -> Option<ColorChoices> {
            use ColorChoices::*;
            Some(match *self {
                Red => Blue,
                Blue => Violet,
                Violet => Red,
            })
        }
    }
    
    impl From<&ColorChoices> for Color {
        fn from(rainbow: &ColorChoices) -> Color {
            use ColorChoices::*;
            match rainbow {
                Red => Color::RED,
                Blue => Color::BLUE,
                Violet => Color::VIOLET,
            }
        }
    }
    
    fn cycle_color(mut query: Query<(&mut ColorChoices, EventReader<CycleColorAction>)>) {
        for (mut color, action_queue) in query.iter_mut() {
            for _ in action_queue.iter() {
                *color = color.next().unwrap();
            }
        }
    }
    
    fn update_text_color(mut query: Query<(&mut Text, &ColorChoices), Changed<ColorChoices>>) {
        for (mut text, rainbow) in query.iter_mut() {
            text.sections[0].style.color = rainbow.into();
        }
    }
    
}

pub mod movement {
    struct MoveAction {
        transform: Transform,
    }
    
    const MOVE_DISTANCE: f32 = 100.0;
    const TIME_STEP: f32 = 2;

    // When events are registered in the AppBuilder using .add_event(), 
    // a system will automatically be created to clean them up after two frames.
    // This can be problematic if your event-consuming systems ever skip frames (such as due to a fixed timestep run criteria).
    // We can get around this by not registering them, and instead handling clean-up by consuming them when read.
    // Be careful though: once consumed, other systems will not be able to read them!
    fn move_text(query: Query<(&mut Transform, EventConsumer<MoveAction>)>) {
        for (mut transform, events) in query.iter() {
            // Unlike EventReaders which simply iterate, EventConsumers drain the events they read
            for move_action in events.drain() {
                *transform += move_action.transform * TIME_STEP;
            }
        }
    }
    
}

pub mod addition {
// Or store data to be responded to
struct AddNumberAction {
    number: u8,
}
// Just as when using Events as a resource, you can work with `Events<T>` directly instead
// EventReader and EventWriter are just convenient wrappers that better communicate intent
// And store state automatically for you
fn add_number(
    mut query: Query<(&mut Text, &Events<AddNumberAction>)>,
    mut reader: Local<ManualEventReader<AddNumberAction>>,
    selected: Res<Selected>,
) {
    let (mut text, action_queue) = query.get_mut(selected.entity).unwrap();
    // Because we only care about one entity at a time, we can store the event reader manually
    // in a Local resource as part of the system's data
    // This logic is handled for you, storing one EventReader per entity when you query for an EventReader
    if selected.is_changed() {
        // If the resource selected is changed, we need to rebuild a new event reader
        *reader = action_queue.get_reader();
    }

    for action in reader.iter(&action_queue) {
        let current_number: u8 = text.sections[0].value.clone().parse().unwrap();
        // Wrap addition, rather than overflowing
        let new_number = ((current_number + action.number) as u16) % std::u8::MAX as u16;
        text.sections[0].value = new_number.to_string();
    }
}

}