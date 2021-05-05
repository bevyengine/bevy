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
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // Adding events using .add_event::<T> will cause all resources and components of type T
        // to be automatically cleaned in a double-buffer fashion by inserting an appropriate system
        //
        // You can avoid this behavior and manually clean up your events by simply adding events
        // as vanilla components or resources
        .add_event::<Message>()
        .add_startup_system(setup.system())
        .add_system(input_dispatch.system())
        .add_system(act.system())
        .run()
}
