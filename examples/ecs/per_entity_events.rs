use bevy::app::Events;
use bevy::prelude::*;

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
///
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
        .init_resource::<Selected>()
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
        .add_system(update_text_color.system().after("action_handling"))
        .run()
}

// Tracks which entity is selected
#[derive(Default)]
struct Selected(Option<Entity>);
// Marks entities as selectable
struct Selectable;
#[derive(Bundle)]
struct InteractableBundle {
    #[bundle]
    text_bundle: Text2dBundle,
    selectable: Selectable,
    rainbow: Rainbow,
    cycle_color_events: Events<CycleColorAction>,
    add_number_events: Events<AddNumberAction>,
}

impl InteractableBundle {
    fn new(x: f32, y: f32, font_handle: &Handle<Font>) -> Self {
        InteractableBundle {
            text_bundle: Text2dBundle {
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
            rainbow: Rainbow::Red,
            cycle_color_events: Events::<CycleColorAction>::default(),
            add_number_events: Events::<AddNumberAction>::default(),
        }
    }
}

enum Rainbow {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
}

impl Iterator for Rainbow {
    type Item = Self;

    fn next(&mut self) -> Option<Rainbow> {
        use Rainbow::*;
        Some(match *self {
            Red => Orange,
            Orange => Yellow,
            Yellow => Green,
            Green => Blue,
            Blue => Violet,
            Violet => Red,
        })
    }
}

impl From<&Rainbow> for Color {
    fn from(rainbow: &Rainbow) -> Color {
        use Rainbow::*;
        match rainbow {
            Red => Color::RED,
            Orange => Color::ORANGE,
            Yellow => Color::YELLOW,
            Green => Color::GREEN,
            Blue => Color::BLUE,
            Violet => Color::VIOLET,
        }
    }
}

// Events can be simple unit structs
struct CycleColorAction;
// Or store data to be responded to
struct AddNumberAction {
    number: u32,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn_bundle(InteractableBundle::new(-200.0, 0.0, &font_handle));
    commands.spawn_bundle(InteractableBundle::new(0.0, 0.0, &font_handle));
    commands.spawn_bundle(InteractableBundle::new(200.0, 0.0, &font_handle));
}

/// Cycles through entities appropriately based on input
fn select_entity(mut query: Query<(Entity, &mut Text), With<Selectable>>, selected: Res<Selected>) {
}

// FIXME: make this work using `EventWriter<T>` syntax and specialized behavior
/// Dispatches actions to entities based on the input
/// Note that we can store several events at once!
/// Try pressing both "Enter" and "Space" at once to cycle colors twice,
/// Or both "1" and "3" to add 4 all at once to the selected display
fn input_dispatch(
    mut query: Query<(
        &'static EventWriter<CycleColorAction>,
        &'static EventWriter<AddNumberAction>,
    )>,
    selected: Res<Selected>,
) {
}

// FIXME: make this work using `EventReader<T>` syntax and specialized behavior
fn cycle_color(mut query: Query<(&mut Rainbow, &'static EventReader<CycleColorAction>)>) {
    for (mut rainbow, cycle_color_action_queue) in query.iter_mut() {
        for _ in cycle_color_action_queue.iter() {
            *rainbow = rainbow.next().unwrap();
        }
    }
}

fn update_text_color(mut query: Query<(&mut Text, &Rainbow), Changed<Rainbow>>) {
    for (mut text, rainbow) in query.iter_mut() {
        text.sections[0].style.color = rainbow.into();
    }
}

// Just as when using Events as a resource, you can work with `Events<T>` directly instead
// EventReader and EventWriter are just convenient wrappers that better communicate intent
fn add_number(mut query: Query<(&mut Text, &Events<AddNumberAction>)>) {
    // To add events manually, use events.send(MyEvent::new())
    for (mut text, action_queue) in query.iter_mut() {
        let mut reader = action_queue.get_reader();
        for action in reader.iter(&action_queue) {
            // TODO: add the number
        }
    }
}
