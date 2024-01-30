//! This example illustrates how to add custom log layers in bevy.

use bevy::{
    log::tracing_subscriber::{layer::SubscriberExt, Layer},
    log::BoxedSubscriber,
    prelude::*,
    utils::tracing::Subscriber,
};

struct CustomLayer;

impl<S: Subscriber> Layer<S> for CustomLayer {
    fn on_event(
        &self,
        event: &bevy::utils::tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        println!("Got event!");
        println!("  level={:?}", event.metadata().level());
        println!("  target={:?}", event.metadata().target());
        println!("  name={:?}", event.metadata().name());
    }
}

fn update_subscriber(subscriber: BoxedSubscriber) -> BoxedSubscriber {
    Box::new(subscriber.with(CustomLayer))
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            update_subscriber: Some(update_subscriber),
            ..default()
        }))
        .add_systems(Update, log_system)
        .run();
}

fn log_system() {
    // here is how you write new logs at each "log level" (in "most import" to
    // "least important" order)
    error!("something failed");
    warn!("something bad happened that isn't a failure, but thats worth calling out");
    info!("helpful information that is worth printing by default");
    debug!("helpful for debugging");
    trace!("very noisy");
}
