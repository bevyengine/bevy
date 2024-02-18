//! Demonstrates using events to send data in and out of a [`SubApp`](bevy_internal::app::SubApp)

use bevy::{
    app::{AppExit, AppLabel, SubApp},
    ecs::event::ManualEventReader,
    prelude::*,
};

/// Events meant to be sent TO the sub app
#[derive(Event, Clone)]
enum ToSubAppEvent {
    HelloFromMainApp,
}

/// Events meant to be recieved FROM the sub app
#[derive(Event, Clone)]
enum FromSubAppEvent {
    HelloFromSubApp,
    PleaseExit,
}

fn main() {
    // We create our app normally
    let mut main_app = App::new();

    // Add whichever plugins we need
    main_app
        .add_plugins((DefaultPlugins, SubAppPlugin))
        // We need to register both events
        .add_event::<ToSubAppEvent>()
        .add_event::<FromSubAppEvent>()
        .add_systems(Update, handle_main_events);

    main_app.run();
}

// This is just a normal system which consumes our events on the main app side
fn handle_main_events(
    // Get events from the sub app
    mut event_reader_from_sub_app: EventReader<FromSubAppEvent>,
    // Write events to the sub app
    mut event_writer_to_sub_app: EventWriter<ToSubAppEvent>,
    // Close the program
    mut event_writer_app_exit: EventWriter<AppExit>,
) {
    // We can iterate over these events like normal
    for event in event_reader_from_sub_app.read() {
        match event {
            // We recieved a "Hello" from the sub app
            FromSubAppEvent::HelloFromSubApp => info!("MainApp: Recieved hello from sub app!"),
            // We recieved a request to close the program
            FromSubAppEvent::PleaseExit => {
                info!("MainApp: Recieved request to exit from sub app!");
                event_writer_app_exit.send(AppExit);
                return;
            }
        }
    }

    // Let's send our own "Hello"
    event_writer_to_sub_app.send(ToSubAppEvent::HelloFromMainApp);
}

// All SubApp's require an AppLabel
#[derive(AppLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SubAppLabel;

// This resource will store the EventReader state for ToSubAppEvents
#[derive(Resource, Default)]
struct EventReaderToState(ManualEventReader<ToSubAppEvent>);

// This resource will store the EventReader state for FromSubAppEvents
#[derive(Resource, Default)]
struct EventReaderFromState(ManualEventReader<FromSubAppEvent>);

// We'll create a plugin for our sub app
struct SubAppPlugin;

impl Plugin for SubAppPlugin {
    fn build(&self, app: &mut App) {
        // We create our app which will become a SubApp of the main app
        let mut our_sub_app = App::new();

        // Add whichever plugins we need
        our_sub_app
            .add_plugins(MinimalPlugins)
            // We need to register both events here as well
            .add_event::<ToSubAppEvent>()
            .add_event::<FromSubAppEvent>()
            .init_resource::<EventReaderToState>()
            .init_resource::<EventReaderFromState>()
            .add_systems(Update, handle_sub_events);

        // Now we need to build the SubApp wrapper around our app
        // We can pass either a closure or a function which takes mutable
        // references main app's world and our sub app's app
        // Note we also setup a post-sync function
        let sub_app = SubApp::new(our_sub_app, pre_sync).with_finish(post_sync);
        // Finally we can insert our SubApp into the main app
        app.insert_sub_app(SubAppLabel, sub_app);
    }
}

// This is just a normal system which consumes our events on the sub app side
// Note we are now reading ToSubAppEvent and writing FromSubAppEvent
fn handle_sub_events(
    // Get events to the sub app
    mut event_reader_to_sub_app: EventReader<ToSubAppEvent>,
    // Write events to the main app
    mut event_writer_from_sub_app: EventWriter<FromSubAppEvent>,
) {
    // We can iterate over these events like normal
    for event in event_reader_to_sub_app.read() {
        match event {
            // We recieved a "Hello" from the main app
            ToSubAppEvent::HelloFromMainApp => {
                info!("SubApp: Recieved hello from main app!");
                // Be polite and say "Hello" back
                event_writer_from_sub_app.send(FromSubAppEvent::HelloFromSubApp);
                // We are done, let's request that the main app closes
                event_writer_from_sub_app.send(FromSubAppEvent::PleaseExit);
            }
        }
    }
}

// This will run before our sub app's schedule allowing for us to extract data from the main world
fn pre_sync(main_world: &mut World, sub_app: &mut App) {
    // Retrieve our event reader state
    sub_app
        .world
        .resource_scope(|sub_world, mut event_reader: Mut<EventReaderToState>| {
            // Retrieve our events
            main_world.resource_scope(|_main_world, events: Mut<Events<ToSubAppEvent>>| {
                // Retrieve our EventWriter to write the events from the main app to the sub app
                sub_world.resource_scope(
                    |_sub_world, mut event_writer: Mut<Events<ToSubAppEvent>>| {
                        for event in event_reader.0.read(&events) {
                            event_writer.send(event.clone());
                        }
                    },
                );
            });
        });
}

// This will run after our sub app's schedule allowing for us to insert data in the main world
fn post_sync(main_world: &mut World, sub_app: &mut App) {
    // Retrieve our event reader state
    sub_app
        .world
        .resource_scope(|sub_world, mut event_reader: Mut<EventReaderFromState>| {
            // Retrieve our events
            sub_world.resource_scope(|_sub_world, events: Mut<Events<FromSubAppEvent>>| {
                // Retrieve our EventWriter to write the events from the sub app to the main app
                main_world.resource_scope(
                    |_main_world, mut event_writer: Mut<Events<FromSubAppEvent>>| {
                        // Read all events from the sub app and pass them to the main app
                        for event in event_reader.0.read(&events) {
                            event_writer.send(event.clone());
                        }
                    },
                );
            });
        });
}
