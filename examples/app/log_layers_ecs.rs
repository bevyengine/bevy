//! This example illustrates how to transfer log events from the [`Layer`] to Bevy's ECS.
//!
//! The way we will do this is via a [`mpsc`] channel. [`mpsc`] channels allow 2 unrelated
//! parts of the program to communicate (in this case, [`Layer`]s and Bevy's ECS).
//!
//! Inside the `custom_layer` function we will create a [`mpsc::Sender`] and a [`mpsc::Receiver`] from a
//! [`mpsc::channel`]. The [`Sender`](mpsc::Sender) will go into the `AdvancedLayer` and the [`Receiver`](mpsc::Receiver) will
//! go into a non-send resource called `LogEvents` (It has to be non-send because [`Receiver`](mpsc::Receiver) is [`!Sync`](Sync)).
//! From there we will use [`transfer_log_messages`] to transfer log messages from [`CapturedLogMessages`] to an ECS message called [`LogMessage`].
//!
//! Finally, after all that we can access the [`LogMessage`] message from our systems and use it.
//! In this example we build a simple log viewer.

use std::sync::mpsc;

use bevy::{
    log::{
        tracing::{self, Subscriber},
        tracing_subscriber::{self, Layer},
        BoxedLayer, Level,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            // Show logs all the way up to the trace level, but only for logs
            // produced by this example.
            level: Level::TRACE,
            filter: "warn,log_layers_ecs=trace".to_string(),
            custom_layer,
            ..default()
        }))
        .add_systems(Startup, (log_system, setup))
        .add_systems(Update, print_logs)
        .run();
}

/// A basic message. This is what we will be sending from the [`CaptureLayer`] to [`CapturedLogMessages`] non-send resource.
#[derive(Debug, Message)]
struct LogMessage {
    message: String,
    level: Level,
}

/// This non-send resource temporarily stores [`LogMessage`]s before they are
/// written to [`Messages<LogEvent>`] by [`transfer_log_messages`].
#[derive(Deref, DerefMut)]
struct CapturedLogMessages(mpsc::Receiver<LogMessage>);

/// Transfers information from the [`CapturedLogMessages`] resource to [`Messages<LogEvent>`](LogMessage).
fn transfer_log_messages(
    receiver: NonSend<CapturedLogMessages>,
    mut message_writer: MessageWriter<LogMessage>,
) {
    // Make sure to use `try_iter()` and not `iter()` to prevent blocking.
    message_writer.write_batch(receiver.try_iter());
}

/// This is the [`Layer`] that we will use to capture log messages and then send them to Bevy's
/// ECS via its [`mpsc::Sender`].
struct CaptureLayer {
    sender: mpsc::Sender<LogMessage>,
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // In order to obtain the log message, we have to create a struct that implements
        // Visit and holds a reference to our string. Then we use the `record` method and
        // the struct to modify the reference to hold the message string.
        let mut message = None;
        event.record(&mut CaptureLayerVisitor(&mut message));
        if let Some(message) = message {
            let metadata = event.metadata();

            self.sender
                .send(LogMessage {
                    message,
                    level: *metadata.level(),
                })
                .expect("LogEvents resource no longer exists!");
        }
    }
}

/// A [`Visit`](tracing::field::Visit)or that records log messages that are transferred to [`CaptureLayer`].
struct CaptureLayerVisitor<'a>(&'a mut Option<String>);
impl tracing::field::Visit for CaptureLayerVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // This if statement filters out unneeded events sometimes show up
        if field.name() == "message" {
            *self.0 = Some(format!("{value:?}"));
        }
    }
}
fn custom_layer(app: &mut App) -> Option<BoxedLayer> {
    let (sender, receiver) = mpsc::channel();

    let layer = CaptureLayer { sender };
    let resource = CapturedLogMessages(receiver);

    app.insert_non_send_resource(resource);
    app.add_message::<LogMessage>();
    app.add_systems(Update, transfer_log_messages);

    Some(layer.boxed())
}

fn log_system() {
    // Here is how you write new logs at each "log level" (in "most important" to
    // "least important" order)
    error!("Something failed");
    warn!("Something bad happened that isn't a failure, but thats worth calling out");
    info!("Helpful information that is worth printing by default");
    debug!("Helpful for debugging");
    trace!("Very noisy");
}

#[derive(Component)]
struct LogViewerRoot;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: vw(100),
            height: vh(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12)),
            ..default()
        },
        LogViewerRoot,
    ));
}

// This is how we can read our LogMessages.
// In this example we are reading the LogMessages and inserting them as text into our log viewer.
fn print_logs(
    mut log_message_reader: MessageReader<LogMessage>,
    mut commands: Commands,
    log_viewer_root: Single<Entity, With<LogViewerRoot>>,
) {
    let root_entity = *log_viewer_root;

    commands.entity(root_entity).with_children(|child| {
        for log_message in log_message_reader.read() {
            child.spawn((
                Text::default(),
                children![
                    (
                        TextSpan::new(format!("{:5} ", log_message.level)),
                        TextColor(level_color(&log_message.level)),
                    ),
                    TextSpan::new(&log_message.message),
                ],
            ));
        }
    });
}

fn level_color(level: &Level) -> Color {
    use bevy::color::palettes::tailwind::*;
    Color::from(match *level {
        Level::WARN => ORANGE_400,
        Level::ERROR => RED_400,
        Level::INFO => GREEN_400,
        Level::TRACE => PURPLE_400,
        Level::DEBUG => BLUE_400,
    })
}
