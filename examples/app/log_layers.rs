//! This example illustrates how to add custom log layers in bevy.

// use bevy::log::tracing_subscriber::Layer;
use bevy::{log::tracing_subscriber::Layer, prelude::*, utils::tracing::field::Visit};
use std::sync::{Arc, Mutex};

struct ChannelLayer(crossbeam_channel::Sender<String>);

#[derive(Resource)]
struct ErrorMessageReceiver(crossbeam_channel::Receiver<String>);

#[derive(Component)]
struct LastErrorText;

impl<S: bevy::utils::tracing::Subscriber> Layer<S> for ChannelLayer {
    fn enabled(
        &self,
        metadata: &bevy::utils::tracing::Metadata<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        // we're only interested in errors
        *metadata.level() <= bevy::log::Level::ERROR
    }

    fn on_event(
        &self,
        event: &bevy::utils::tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        // log received events to std err, including metadata
        eprintln!("logged my way: {event:#?}");

        let mut visitor = Visitor(self.0.clone());
        event.record(&mut visitor);
    }
}

struct Visitor(crossbeam_channel::Sender<String>);

impl Visit for Visitor {
    fn record_debug(
        &mut self,
        _field: &bevy::utils::tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        // will fail if the receiver is dropped. In that case, we do nothing.
        _ = self.0.try_send(format!("{value:?}"));
    }
}

fn main() {
    let (sender, receiver) = crossbeam_channel::unbounded();

    App::new()
        .insert_resource(ErrorMessageReceiver(receiver))
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            // todo: fix horrible hack
            extra_layers: Arc::new(Mutex::new(Some(vec![Box::new(ChannelLayer(sender))]))),
            ..default()
        }))
        .add_startup_system(log_system)
        .add_startup_system(text_setup)
        .add_system(update_screen_text)
        .run();
}

fn text_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        TextBundle::from_sections([TextSection::from_style(TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 60.0,
            color: Color::MAROON,
        })]),
        LastErrorText,
    ));
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

fn update_screen_text(
    errors: Res<ErrorMessageReceiver>,
    mut query: Query<&mut Text, With<LastErrorText>>,
) {
    for error in errors.0.try_iter() {
        if let Ok(mut text) = query.get_single_mut() {
            text.sections[0].value = error;
        }
    }
}
