//! This example illustrates how to add custom log layers in bevy.

use bevy::{
    log::tracing_subscriber::Layer,
    prelude::*,
    utils::tracing::{field::Visit, Subscriber},
};
use std::sync::{Arc, Mutex};

struct DebugLogToStdErrLayer;

impl<S: Subscriber> Layer<S> for DebugLogToStdErrLayer {
    fn on_event(
        &self,
        event: &bevy::utils::tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        // pretty print received events to std err, including metadata
        eprintln!("logged my way: {event:#?}");
    }
}

struct ChannelLayer {
    sender: crossbeam_channel::Sender<String>,
    max_level: bevy::log::Level,
}

#[derive(Resource)]
struct ErrorMessageReceiver(crossbeam_channel::Receiver<String>);

#[derive(Component)]
struct LastErrorText;

impl<S: Subscriber> Layer<S> for ChannelLayer {
    fn on_event(
        &self,
        event: &bevy::utils::tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        if event.metadata().level() <= &self.max_level {
            let mut visitor = ChannelSendVisitor(self.sender.clone());
            event.record(&mut visitor);
        }
    }
}

struct ChannelSendVisitor(crossbeam_channel::Sender<String>);

impl Visit for ChannelSendVisitor {
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

    let log_to_screen_layer = ChannelLayer {
        sender,
        max_level: bevy::log::Level::ERROR,
    };

    App::new()
        .insert_resource(ErrorMessageReceiver(receiver))
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            // todo: fix horrible hack
            extra_layers: Arc::new(Mutex::new(Some(vec![
                log_to_screen_layer.boxed(),
                DebugLogToStdErrLayer.boxed(),
            ]))),
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
