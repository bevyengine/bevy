//! This example illustrates how to add custom log layers in bevy.

use bevy::{
    log::{
        tracing::{self, Subscriber},
        tracing_subscriber::Layer,
        BoxedFmtLayer, BoxedLayer,
    },
    prelude::*,
};

struct CustomLayer;

impl<S: Subscriber> Layer<S> for CustomLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        println!("Got event!");
        println!("  level={}", event.metadata().level());
        println!("  target={}", event.metadata().target());
        println!("  name={}", event.metadata().name());
    }
}

// We don't need App for this example, as we are just printing log information.
// For an example that uses App, see log_layers_ecs.
fn custom_layer(_app: &mut App) -> Option<BoxedLayer> {
    // You can provide multiple layers like this, since Vec<Layer> is also a layer:
    Some(Box::new(vec![
        bevy::log::tracing_subscriber::fmt::layer()
            .with_file(true)
            .boxed(),
        CustomLayer.boxed(),
    ]))
}

// While `custom_layer` allows you to add _additional_ layers, it won't allow you to override the
// default `tracing_subscriber::fmt::Layer` added by `LogPlugin`. To do that, you can use the
// `fmt_layer` option.
//
// In this example, we're disabling the timestamp in the log output.
fn fmt_layer(_app: &mut App) -> Option<BoxedFmtLayer> {
    Some(Box::new(
        bevy::log::tracing_subscriber::fmt::Layer::default()
            .without_time()
            .with_writer(std::io::stderr),
    ))
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            custom_layer,
            fmt_layer,

            ..default()
        }))
        .add_systems(Update, log_system)
        .run();
}

fn log_system() {
    // here is how you write new logs at each "log level" (in "most important" to
    // "least important" order)
    error!("something failed");
    warn!("something bad happened that isn't a failure, but thats worth calling out");
    info!("helpful information that is worth printing by default");
    debug!("helpful for debugging");
    trace!("very noisy");
}
