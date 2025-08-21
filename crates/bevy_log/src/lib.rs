#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! This crate provides logging functions and configuration for [Bevy](https://bevy.org)
//! apps, and automatically configures platform specific log handlers (i.e. Wasm or Android).
//!
//! The macros provided for logging are reexported from [`tracing`](https://docs.rs/tracing),
//! and behave identically to it.
//!
//! By default, the [`LogPlugin`] from this crate is included in Bevy's `DefaultPlugins`
//! and the logging macros can be used out of the box, if used.
//!
//! For more fine-tuned control over logging behavior, set up the [`LogPlugin`] or
//! `DefaultPlugins` during app initialization.

extern crate alloc;

use core::error::Error;

#[cfg(target_os = "android")]
mod android_tracing;
mod once;

#[cfg(feature = "trace_tracy_memory")]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

/// The log prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use tracing::{
        debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    };

    #[doc(hidden)]
    pub use crate::{debug_once, error_once, info_once, trace_once, warn_once};

    #[doc(hidden)]
    pub use bevy_utils::once;
}

pub use bevy_utils::once;
pub use tracing::{
    self, debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn,
    warn_span, Level,
};
pub use tracing_subscriber;

use bevy_app::{App, Plugin};
use tracing_log::LogTracer;
use tracing_subscriber::{
    filter::{FromEnvError, ParseError},
    layer::Layered,
    prelude::*,
    registry::Registry,
    EnvFilter, Layer,
};
#[cfg(feature = "tracing-chrome")]
use {
    bevy_ecs::resource::Resource,
    bevy_platform::cell::SyncCell,
    tracing_subscriber::fmt::{format::DefaultFields, FormattedFields},
};

/// Wrapper resource for `tracing-chrome`'s flush guard.
/// When the guard is dropped the chrome log is written to file.
#[cfg(feature = "tracing-chrome")]
#[expect(
    dead_code,
    reason = "`FlushGuard` never needs to be read, it just needs to be kept alive for the `App`'s lifetime."
)]
#[derive(Resource)]
pub(crate) struct FlushGuard(SyncCell<tracing_chrome::FlushGuard>);

/// Adds logging to Apps. This plugin is part of the `DefaultPlugins`. Adding
/// this plugin will setup a collector appropriate to your target platform:
/// * Using [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber) by default,
///   logging to `stdout`.
/// * Using [`android_log-sys`](https://crates.io/crates/android_log-sys) on Android,
///   logging to Android logs.
/// * Using [`tracing-wasm`](https://crates.io/crates/tracing-wasm) in Wasm, logging
///   to the browser console.
///
/// You can configure this plugin.
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_log::LogPlugin;
/// # use tracing::Level;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.set(LogPlugin {
///             level: Level::DEBUG,
///             filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
///             custom_layer: |_| None,
///             fmt_layer: |_| None,
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
/// Also, to disable color terminal output (ANSI escape codes), you can
/// set the environment variable `NO_COLOR` to any value. This common
/// convention is documented at [no-color.org](https://no-color.org/).
/// For example:
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_log::LogPlugin;
/// fn main() {
/// #   // SAFETY: Single-threaded
/// #   unsafe {
///     std::env::set_var("NO_COLOR", "1");
/// #   }
///     App::new()
///        .add_plugins(DefaultPlugins)
///        .run();
/// }
/// ```
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
/// # Example Setup
///
/// For a quick setup that enables all first-party logging while not showing any of your dependencies'
/// log data, you can configure the plugin as shown below.
///
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_log::*;
/// App::new()
///     .add_plugins(DefaultPlugins.set(LogPlugin {
///         filter: "warn,my_crate=trace".to_string(), //specific filters
///         level: Level::TRACE,//Change this to be globally change levels
///         ..Default::default()
///         }))
///     .run();
/// ```
/// The filter (in this case an `EnvFilter`) chooses whether to print the log. The most specific filters apply with higher priority.
/// Let's start with an example: `filter: "warn".to_string()` will only print logs with level `warn` level or greater.
/// From here, we can change to `filter: "warn,my_crate=trace".to_string()`. Logs will print at level `warn` unless it's in `mycrate`,
/// which will instead print at `trace` level because `my_crate=trace` is more specific.
///
///
/// ## Log levels
/// Events can be logged at various levels of importance.
/// Only events at your configured log level and higher will be shown.
/// ```no_run
/// # use bevy_log::*;
/// // here is how you write new logs at each "log level" (in "most important" to
/// // "least important" order)
/// error!("something failed");
/// warn!("something bad happened that isn't a failure, but that's worth calling out");
/// info!("helpful information that is worth printing by default");
/// debug!("helpful for debugging");
/// trace!("very noisy");
/// ```
/// In addition to `format!` style arguments, you can print a variable's debug
/// value by using syntax like: `trace(?my_value)`.
///
/// ## Per module logging levels
/// Modules can have different logging levels using syntax like `crate_name::module_name=debug`.
///
///
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_log::*;
/// App::new()
///     .add_plugins(DefaultPlugins.set(LogPlugin {
///         filter: "warn,my_crate=trace,my_crate::my_module=debug".to_string(), // Specific filters
///         level: Level::TRACE, // Change this to be globally change levels
///         ..Default::default()
///     }))
///     .run();
/// ```
/// The idea is that instead of deleting logs when they are no longer immediately applicable,
/// you just disable them. If you do need to log in the future, then you can enable the logs instead of having to rewrite them.
///
/// ## Further reading
///
/// The `tracing` crate has much more functionality than these examples can show.
/// Much of this configuration can be done with "layers" in the `log` crate.
/// Check out:
/// - Using spans to add more fine grained filters to logs
/// - Adding instruments to capture more function information
/// - Creating layers to add additional context such as line numbers
/// # Panics
///
/// This plugin should not be added multiple times in the same process. This plugin
/// sets up global logging configuration for **all** Apps in a given process, and
/// rerunning the same initialization multiple times will lead to a panic.
///
/// # Performance
///
/// Filters applied through this plugin are computed at _runtime_, which will
/// have a non-zero impact on performance.
/// To achieve maximum performance, consider using
/// [_compile time_ filters](https://docs.rs/log/#compile-time-filters)
/// provided by the [`log`](https://crates.io/crates/log) crate.
///
/// ```toml
/// # cargo.toml
/// [dependencies]
/// log = { version = "0.4", features = ["max_level_debug", "release_max_level_warn"] }
/// ```
pub struct LogPlugin {
    /// Filters logs using the [`EnvFilter`] format
    pub filter: String,

    /// Filters out logs that are "less than" the given level.
    /// This can be further filtered using the `filter` setting.
    pub level: Level,

    /// Optionally add an extra [`Layer`] to the tracing subscriber
    ///
    /// This function is only called once, when the plugin is built.
    ///
    /// Because [`BoxedLayer`] takes a `dyn Layer`, `Vec<Layer>` is also an acceptable return value.
    ///
    /// Access to [`App`] is also provided to allow for communication between the
    /// [`Subscriber`](tracing::Subscriber) and the [`App`].
    ///
    /// Please see the `examples/log_layers.rs` for a complete example.
    pub custom_layer: fn(app: &mut App) -> Option<BoxedLayer>,

    /// Override the default [`tracing_subscriber::fmt::Layer`] with a custom one.
    ///
    /// This differs from [`custom_layer`](Self::custom_layer) in that
    /// [`fmt_layer`](Self::fmt_layer) allows you to overwrite the default formatter layer, while
    /// `custom_layer` only allows you to add additional layers (which are unable to modify the
    /// default formatter).
    ///
    /// For example, you can use [`tracing_subscriber::fmt::Layer::without_time`] to remove the
    /// timestamp from the log output.
    ///
    /// Please see the `examples/log_layers.rs` for a complete example.
    pub fmt_layer: fn(app: &mut App) -> Option<BoxedFmtLayer>,
}

/// A boxed [`Layer`] that can be used with [`LogPlugin::custom_layer`].
pub type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

#[cfg(feature = "trace")]
type BaseSubscriber =
    Layered<EnvFilter, Layered<Option<Box<dyn Layer<Registry> + Send + Sync>>, Registry>>;

#[cfg(feature = "trace")]
type PreFmtSubscriber = Layered<tracing_error::ErrorLayer<BaseSubscriber>, BaseSubscriber>;

#[cfg(not(feature = "trace"))]
type PreFmtSubscriber =
    Layered<EnvFilter, Layered<Option<Box<dyn Layer<Registry> + Send + Sync>>, Registry>>;

/// A boxed [`Layer`] that can be used with [`LogPlugin::fmt_layer`].
pub type BoxedFmtLayer = Box<dyn Layer<PreFmtSubscriber> + Send + Sync + 'static>;

/// The default [`LogPlugin`] [`EnvFilter`].
pub const DEFAULT_FILTER: &str = "wgpu=error,naga=warn";

impl Default for LogPlugin {
    fn default() -> Self {
        Self {
            filter: DEFAULT_FILTER.to_string(),
            level: Level::INFO,
            custom_layer: |_| None,
            fmt_layer: |_| None,
        }
    }
}

impl Plugin for LogPlugin {
    #[expect(clippy::print_stderr, reason = "Allowed during logger setup")]
    fn build(&self, app: &mut App) {
        #[cfg(feature = "trace")]
        {
            let old_handler = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |infos| {
                eprintln!("{}", tracing_error::SpanTrace::capture());
                old_handler(infos);
            }));
        }

        let finished_subscriber;
        let subscriber = Registry::default();

        // add optional layer provided by user
        let subscriber = subscriber.with((self.custom_layer)(app));

        let default_filter = { format!("{},{}", self.level, self.filter) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|from_env_error| {
                _ = from_env_error
                    .source()
                    .and_then(|source| source.downcast_ref::<ParseError>())
                    .map(|parse_err| {
                        // we cannot use the `error!` macro here because the logger is not ready yet.
                        eprintln!("LogPlugin failed to parse filter from env: {parse_err}");
                    });

                Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(&default_filter))
            })
            .unwrap();
        let subscriber = subscriber.with(filter_layer);

        #[cfg(feature = "trace")]
        let subscriber = subscriber.with(tracing_error::ErrorLayer::default());

        #[cfg(all(
            not(target_arch = "wasm32"),
            not(target_os = "android"),
            not(target_os = "ios")
        ))]
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
                app.insert_resource(FlushGuard(SyncCell::new(guard)));
                chrome_layer
            };

            #[cfg(feature = "tracing-tracy")]
            let tracy_layer = tracing_tracy::TracyLayer::default();

            let fmt_layer = (self.fmt_layer)(app).unwrap_or_else(|| {
                // note: the implementation of `Default` reads from the env var NO_COLOR
                // to decide whether to use ANSI color codes, which is common convention
                // https://no-color.org/
                Box::new(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
            });

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
            finished_subscriber = subscriber.with(tracing_wasm::WASMLayer::new(
                tracing_wasm::WASMLayerConfig::default(),
            ));
        }

        #[cfg(target_os = "android")]
        {
            finished_subscriber = subscriber.with(android_tracing::AndroidLayer::default());
        }

        #[cfg(target_os = "ios")]
        {
            finished_subscriber = subscriber.with(tracing_oslog::OsLogger::default());
        }

        let logger_already_set = LogTracer::init().is_err();
        let subscriber_already_set =
            tracing::subscriber::set_global_default(finished_subscriber).is_err();

        match (logger_already_set, subscriber_already_set) {
            (true, true) => error!(
                "Could not set global logger and tracing subscriber as they are already set. Consider disabling LogPlugin."
            ),
            (true, false) => error!("Could not set global logger as it is already set. Consider disabling LogPlugin."),
            (false, true) => error!("Could not set global tracing subscriber as it is already set. Consider disabling LogPlugin."),
            (false, false) => (),
        }
    }
}
