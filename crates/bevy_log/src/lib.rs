#![warn(missing_docs)]
//! This crate provides logging functions and configuration for [Bevy](https://bevyengine.org)
//! apps, and automatically configures platform specific log handlers (i.e. WASM or Android).
//!
//! The macros provided for logging are reexported from [`tracing`](https://docs.rs/tracing),
//! and behave identically to it.
//!
//! By default, the [`LogPlugin`] from this crate is included in Bevy's `DefaultPlugins`
//! and the logging macros can be used out of the box, if used.
//!
//! For more fine-tuned control over logging behavior, set up the [`LogPlugin`] or
//! `DefaultPlugins` during app initialization.

mod once;

#[cfg(feature = "trace")]
use std::panic;
use std::{
    sync::{Arc, Mutex},
    time::SystemTime,
};

#[cfg(target_os = "android")]
mod android_tracing;

#[cfg(feature = "trace_tracy_memory")]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

pub mod prelude {
    //! The Bevy Log Prelude.
    #[doc(hidden)]
    pub use bevy_utils::tracing::{
        debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    };

    pub use crate::{debug_once, error_once, info_once, trace_once, warn_once};
}

use bevy_ecs::{
    event::{Event, EventWriter},
    system::{Res, Resource},
};
use bevy_utils::tracing::Subscriber;
pub use bevy_utils::tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    Level,
};

use bevy_app::{App, Plugin, Update};
use tracing_log::LogTracer;
#[cfg(feature = "tracing-chrome")]
use tracing_subscriber::fmt::{format::DefaultFields, FormattedFields};
use tracing_subscriber::{field::Visit, layer::Layer, prelude::*, registry::Registry, EnvFilter};

/// Adds logging to Apps. This plugin is part of the `DefaultPlugins`. Adding
/// this plugin will setup a collector appropriate to your target platform:
/// * Using [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber) by default,
/// logging to `stdout`.
/// * Using [`android_log-sys`](https://crates.io/crates/android_log-sys) on Android,
/// logging to Android logs.
/// * Using [`tracing-wasm`](https://crates.io/crates/tracing-wasm) in WASM, logging
/// to the browser console.
///
/// You can configure this plugin.
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_log::LogPlugin;
/// # use bevy_utils::tracing::Level;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.set(LogPlugin {
///             level: Level::DEBUG,
///             filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
///         }))
///         .run();
/// }
/// ```
///
/// Log level can also be changed using the `RUST_LOG` environment variable.
/// For example, using `RUST_LOG=wgpu=error,bevy_render=info,bevy_ecs=trace cargo run ..`
///
/// It has the same syntax as the field [`LogPlugin::filter`], see [`EnvFilter`].
/// If you define the `RUST_LOG` environment variable, the [`LogPlugin`] settings
/// will be ignored.
///
/// If you want to setup your own tracing collector, you should disable this
/// plugin from `DefaultPlugins`:
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_log::LogPlugin;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
///         .run();
/// }
/// ```
///
/// # Panics
///
/// This plugin should not be added multiple times in the same process. This plugin
/// sets up global logging configuration for **all** Apps in a given process, and
/// rerunning the same initialization multiple times will lead to a panic.
pub struct LogPlugin {
    /// Filters logs using the [`EnvFilter`] format
    pub filter: String,

    /// Filters out logs that are "less than" the given level.
    /// This can be further filtered using the `filter` setting.
    pub level: Level,
}

impl Default for LogPlugin {
    fn default() -> Self {
        Self {
            filter: "wgpu=error,naga=warn".to_string(),
            level: Level::INFO,
        }
    }
}

impl Plugin for LogPlugin {
    #[cfg_attr(not(feature = "tracing-chrome"), allow(unused_variables))]
    fn build(&self, app: &mut App) {
        #[cfg(feature = "trace")]
        {
            let old_handler = panic::take_hook();
            panic::set_hook(Box::new(move |infos| {
                eprintln!("{}", tracing_error::SpanTrace::capture());
                old_handler(infos);
            }));
        }

        let finished_subscriber;
        let default_filter = { format!("{},{}", self.level, self.filter) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&default_filter))
            .unwrap();

        // Log events
        let log_events = LogEvents::default();
        let log_event_resource = LogEventResource(log_events.clone());
        let log_event_layer = LogEventLayer(log_events);

        app.insert_resource(log_event_resource)
            .add_event::<LogEvent>()
            .add_systems(Update, transfer_log_events);

        let subscriber = Registry::default().with(filter_layer).with(log_event_layer);

        #[cfg(feature = "trace")]
        let subscriber = subscriber.with(tracing_error::ErrorLayer::default());

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            #[cfg(feature = "tracing-chrome")]
            let chrome_layer = {
                let mut layer = tracing_chrome::ChromeLayerBuilder::new();
                if let Ok(path) = std::env::var("TRACE_CHROME") {
                    layer = layer.file(path);
                }
                let (chrome_layer, guard) = layer
                    .name_fn(Box::new(|event_or_span| match event_or_span {
                        tracing_chrome::EventOrSpan::Event(event) => event.metadata().name().into(),
                        tracing_chrome::EventOrSpan::Span(span) => {
                            if let Some(fields) =
                                span.extensions().get::<FormattedFields<DefaultFields>>()
                            {
                                format!("{}: {}", span.metadata().name(), fields.fields.as_str())
                            } else {
                                span.metadata().name().into()
                            }
                        }
                    }))
                    .build();
                app.world.insert_non_send_resource(guard);
                chrome_layer
            };

            #[cfg(feature = "tracing-tracy")]
            let tracy_layer = tracing_tracy::TracyLayer::new();

            let fmt_layer = tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr);

            // bevy_render::renderer logs a `tracy.frame_mark` event every frame
            // at Level::INFO. Formatted logs should omit it.
            #[cfg(feature = "tracing-tracy")]
            let fmt_layer =
                fmt_layer.with_filter(tracing_subscriber::filter::FilterFn::new(|meta| {
                    meta.fields().field("tracy.frame_mark").is_none()
                }));

            let subscriber = subscriber.with(fmt_layer);

            #[cfg(feature = "tracing-chrome")]
            let subscriber = subscriber.with(chrome_layer);
            #[cfg(feature = "tracing-tracy")]
            let subscriber = subscriber.with(tracy_layer);

            finished_subscriber = subscriber;
        }

        #[cfg(target_arch = "wasm32")]
        {
            console_error_panic_hook::set_once();
            finished_subscriber = subscriber.with(tracing_wasm::WASMLayer::new(
                tracing_wasm::WASMLayerConfig::default(),
            ));
        }

        #[cfg(target_os = "android")]
        {
            finished_subscriber = subscriber.with(android_tracing::AndroidLayer::default());
        }

        let logger_already_set = LogTracer::init().is_err();
        let subscriber_already_set =
            bevy_utils::tracing::subscriber::set_global_default(finished_subscriber).is_err();

        match (logger_already_set, subscriber_already_set) {
            (true, true) => warn!(
                "Could not set global logger and tracing subscriber as they are already set. Consider disabling LogPlugin."
            ),
            (true, _) => warn!("Could not set global logger as it is already set. Consider disabling LogPlugin."),
            (_, true) => warn!("Could not set global tracing subscriber as it is already set. Consider disabling LogPlugin."),
            _ => (),
        }
    }
}

/// A [`tracing`](bevy_utils::tracing) log message event.
///
/// This event is helpful for creating custom log viewing systems such as consoles and terminals.
#[derive(Event, Debug, Clone)]
pub struct LogEvent {
    /// The message contents.
    pub message: String,
    /// The name of the span described by this metadata.
    pub name: &'static str,
    /// The part of the system that the span that this metadata describes
    /// occurred in.
    pub target: &'static str,

    /// The level of verbosity of the described span.
    pub level: Level,

    /// The name of the Rust module where the span occurred, or `None` if this
    /// could not be determined.
    pub module_path: Option<&'static str>,

    /// The name of the source code file where the span occurred, or `None` if
    /// this could not be determined.
    pub file: Option<&'static str>,

    /// The line number in the source code file where the span occurred, or
    /// `None` if this could not be determined.
    pub line: Option<u32>,

    /// The time the log occurred.
    /// It is recommended to use a third-party crate like
    /// [chrono](https://crates.io/crates/chrono) to format the [`SystemTime`].
    pub time: SystemTime,
}

/// Transfers information from the [`LogEventResource`] resource to [`Events<LogEvent>`](bevy_ecs::event::Events<LogEvent>).
fn transfer_log_events(handler: Res<LogEventResource>, mut log_events: EventWriter<LogEvent>) {
    let events = &mut *handler.0.lock().unwrap();
    if !events.is_empty() {
        log_events.send_batch(std::mem::take(events));
    }
}

/// This type holds an [`Arc`], of which there are 2 instances.
/// One in the ECS (stored in [`LogEventResource`]), and one in the [`LogEventLayer`].
///
/// This allows the [`LogEventLayer`] to send [`LogEvent`]s to [`LogEventResource`].
type LogEvents = Arc<Mutex<Vec<LogEvent>>>;

/// This resource temporarily stores [`LogEvent`]s before they are
/// written to [`EventWriter<LogEvent>`] by [`transfer_log_events`].
///
/// Read the docs of [`LogEvents`] for more.
#[derive(Resource)]
struct LogEventResource(LogEvents);

/// A tracing [`Layer`] that captures log events and saves them to [`LogEventResource`] via [`LogEvents`].
struct LogEventLayer(LogEvents);
impl<S: Subscriber> Layer<S> for LogEventLayer {
    fn on_event(
        &self,
        event: &bevy_utils::tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut message = None;
        event.record(&mut LogEventVisitor(&mut message));
        if let Some(message) = message {
            let metadata = event.metadata();
            self.0.lock().unwrap().push(LogEvent {
                message,
                name: metadata.name(),
                target: metadata.target(),
                level: *metadata.level(),
                module_path: metadata.module_path(),
                file: metadata.file(),
                line: metadata.line(),
                time: SystemTime::now(),
            });
        }
    }
}

/// A [`Visit`]or that records log events that are transfered to [`LogEventLayer`].
struct LogEventVisitor<'a>(&'a mut Option<String>);
impl<'a> Visit for LogEventVisitor<'a> {
    fn record_debug(
        &mut self,
        field: &bevy_utils::tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        // Only log out messages
        if field.name() == "message" {
            *self.0 = Some(format!("{value:?}"));
        }
    }
}
