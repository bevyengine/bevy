//! This example illustrates how to add custom log layers in bevy.

// use bevy::log::tracing_subscriber::Layer;
use bevy::{
    log::tracing_subscriber::Layer,
    prelude::*,
    utils::tracing::{field::Visit, span},
};
use std::sync::{Arc, Mutex};

struct ShowErrorLayer(crossbeam_channel::Sender<String>);

#[derive(Resource)]
struct ErrorMessageReceiver(crossbeam_channel::Receiver<String>);

#[derive(Component)]
struct LastErrorText;

impl<S: bevy::utils::tracing::Subscriber> Layer<S> for ShowErrorLayer {
    fn on_event(
        &self,
        event: &bevy::utils::tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        eprintln!("log events handled my way!: {event:#?}");
        if let &bevy::log::Level::ERROR = event.metadata().level() {
            let mut visitor = Visitor(self.0.clone());
            event.record(&mut visitor);
            // event.record(|field| {});
            // if let Some(first_record) = event.fields().next() {
            // self.0.send(first_record).unwrap();
            // }
        }
    }
}

struct Visitor(crossbeam_channel::Sender<String>);

impl Visit for Visitor {
    fn record_error(
        &mut self,
        _field: &bevy::utils::tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        // self.record_debug(field, &bevy::utils::tracing::field::DisplayValue(value))
        self.0.send(value.to_string());
    }

    fn record_debug(
        &mut self,
        _field: &bevy::utils::tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        self.0.send(format!("{value:?}"));
        // self.record_debug(field, &value)
    }
}

fn main() {
    let (sender, receiver) = crossbeam_channel::unbounded();

    App::new()
        .insert_resource(ErrorMessageReceiver(receiver))
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            // todo: fix horrible hack
            extra_layers: Arc::new(Mutex::new(Some(vec![Box::new(ShowErrorLayer(sender))]))),
            ..default()
        }))
        .add_startup_system(log_system)
        .add_startup_system(text_setup)
        .add_system(text_update)
        .run();
}

fn text_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2dBundle::default());

    // Text with multiple sections
    commands.spawn((
        // Create a TextBundle that has a Text with a list of sections.
        TextBundle::from_sections([TextSection::from_style(TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 60.0,
            color: Color::RED,
        })]),
        LastErrorText,
    ));
}

fn log_system() {
    // here is how you write new logs at each "log level" (in "least important" to "most important"
    // order)
    trace!("very noisy");
    debug!("helpful for debugging");
    info!("helpful information that is worth printing by default");
    warn!("something bad happened that isn't a failure, but thats worth calling out");
    error!("something failed");

    // by default, trace and debug logs are ignored because they are "noisy"
    // you can control what level is logged by setting up the LogPlugin
    // alternatively you can set the log level via the RUST_LOG=LEVEL environment variable
    // ex: RUST_LOG=trace, RUST_LOG=info,bevy_ecs=warn
    // the format used here is super flexible. check out this documentation for more info:
    // https://docs.rs/tracing-subscriber/*/tracing_subscriber/filter/struct.EnvFilter.html
}

fn text_update(
    errors: Res<ErrorMessageReceiver>,
    mut query: Query<&mut Text, With<LastErrorText>>,
) {
    for error in errors.0.try_iter() {
        if let Ok(mut text) = query.get_single_mut() {
            text.sections[0].value = error;
        }
    }
}
