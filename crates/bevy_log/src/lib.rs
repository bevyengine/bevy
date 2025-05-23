#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate provides logging functions and configuration for [Bevy](https://bevyengine.org)
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
    prelude::*,
    registry::Registry,
    EnvFilter, Layer,
};
#[cfg(feature = "tracing-chrome")]
use {
    bevy_ecs::resource::Resource,
    bevy_utils::synccell::SyncCell,
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
/// * Using [`tracing_oslog`](https://crates.io/crates/tracing_oslog) on iOS,
///   logging to iOS logs.
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
    ///
    /// Note that this field has no effect when `os_target` is `android`, `ios` or `wasm`, as on those
    /// platforms we don't use [`tracing_subscriber::fmt::Layer`] but rather the platform default.
    pub fmt_layer: fn(app: &mut App) -> Option<BoxedLayer>,
}

/// A boxed [`Layer`] that can be used with [`LogPlugin::custom_layer`].
pub type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

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

        // We use a Vec of BoxedLayer instead of adding each layer individually using the
        // `layer.with(next_layer)`.
        // Otherwise, the types of each successive layer becomes unwieldy,
        // as the type of each new layer would depend on the types of the previous layers.
        //
        let mut layers: Vec<BoxedLayer> = Vec::new();

        // Add optional layer provided by user
        // As they are added first, any of the following layers won't be applied.
        // In particular, it won't be affected by the filtering we put in place next.
        if let Some(layer) = (self.custom_layer)(app) {
            layers.push(layer);
        }

        layers.push(Self::build_filter_layer(self.level, &self.filter));

        #[cfg(feature = "trace")]
        layers.push(tracing_error::ErrorLayer::default().boxed());

        layers.push(Self::build_system_output_layer((self.fmt_layer)(app)));

        #[cfg(all(
            not(target_arch = "wasm32"),
            not(target_os = "android"),
            not(target_os = "ios")
        ))]
        {
            #[cfg(feature = "tracing-chrome")]
            {
                let (chrome_layer, guard) = Self::build_chrome_layer();
                app.insert_resource(FlushGuard(SyncCell::new(guard)));
                layers.push(chrome_layer);
            }
            #[cfg(feature = "tracing-tracy")]
            layers.push(tracing_tracy::TracyLayer::default().boxed());
        }

        let subscriber = Registry::default().with(layers);

        let logger_already_set = LogTracer::init().is_err();
        let subscriber_already_set = tracing::subscriber::set_global_default(subscriber).is_err();

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

impl LogPlugin {
    /// Build a [`BoxedLayer`] that will filter which logs are outputted.
    /// It will read the `RUST_LOG` env variable to override the settings
    /// on a given run, the default will fallback to the provided `level` and `filter`
    fn build_filter_layer(level: Level, filter: &str) -> BoxedLayer {
        let default_filter = { format!("{},{}", level, filter) };

        EnvFilter::try_from_default_env()
            .or_else(|from_env_error| {
                _ = from_env_error
                    .source()
                    .and_then(|source| source.downcast_ref::<ParseError>())
                    .map(|parse_err| {
                        #[expect(
                            clippy::print_stderr,
                            reason = "We cannot use the `error!` macro here because the logger is not ready yet."
                        )]
                        {
                        eprintln!("LogPlugin failed to parse filter from env: {}", parse_err);
                        }
                    });

                Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(&default_filter))
            })
            .unwrap().boxed()
    }

    #[cfg(feature = "tracing-chrome")]
    /// [`BoxedLayer`] to build the necessary output when the `tracing-chrome` feature is enabled.
    /// The [`tracing_chrome::FlushGuard`] must be kept around till we don't need to output logs
    /// any more
    fn build_chrome_layer() -> (BoxedLayer, tracing_chrome::FlushGuard) {
        let mut layer = tracing_chrome::ChromeLayerBuilder::new();
        if let Ok(path) = std::env::var("TRACE_CHROME") {
            layer = layer.file(path);
        }
        let (chrome_layer, guard) = layer
            .name_fn(Box::new(|event_or_span| match event_or_span {
                tracing_chrome::EventOrSpan::Event(event) => event.metadata().name().into(),
                tracing_chrome::EventOrSpan::Span(span) => {
                    if let Some(fields) = span.extensions().get::<FormattedFields<DefaultFields>>()
                    {
                        format!("{}: {}", span.metadata().name(), fields.fields.as_str())
                    } else {
                        span.metadata().name().into()
                    }
                }
            }))
            .build();
        (chrome_layer.boxed(), guard)
    }

    #[expect(
        clippy::allow_attributes,
        reason = "We can't switch to `expect` for allow(unused_variables) as we use it if not on those platforms"
    )]
    #[allow(unused_variables, reason = "Not used on `wasm32`, `android` or `ios")]
    /// Build a [`BoxedLayer`] that outputs logs to the system default.
    /// On most platforms, it will be `stderr` with [`tracing_subscriber::fmt::Layer`], expect on `android`, `ios` and` wasm32` where it
    /// uses those system default log infrastructure.
    /// It is possible to override how you output those logs by providing a `custom_format_layer`.
    /// Note that won't have an effect on platform that don't use [`tracing_subscriber::fmt::Layer`]
    fn build_system_output_layer(custom_format_layer: Option<BoxedLayer>) -> BoxedLayer {
        let layer: BoxedLayer;
        #[cfg(target_arch = "wasm32")]
        {
            layer = tracing_wasm::WASMLayer::new(tracing_wasm::WASMLayerConfig::default()).boxed();
        }

        #[cfg(target_os = "android")]
        {
            layer = android_tracing::AndroidLayer::default().boxed();
        }

        #[cfg(target_os = "ios")]
        {
            layer = tracing_oslog::OsLogger::default().boxed();
        }

        #[cfg(all(
            not(target_arch = "wasm32"),
            not(target_os = "android"),
            not(target_os = "ios")
        ))]
        {
            layer = {
                let fmt_layer = custom_format_layer.unwrap_or_else(|| {
                    tracing_subscriber::fmt::Layer::default()
                        // note: the implementation of `Default` reads from the env var NO_COLOR
                        // to decide whether to use ANSI color codes, which is common convention
                        // https://no-color.org/
                        .with_writer(std::io::stderr)
                        .boxed()
                });

                // bevy_render::renderer logs a `tracy.frame_mark` event every frame
                // at Level::INFO. Formatted logs should omit it.
                #[cfg(feature = "tracing-tracy")]
                let fmt_layer =
                    fmt_layer.with_filter(tracing_subscriber::filter::FilterFn::new(|meta| {
                        meta.fields().field("tracy.frame_mark").is_none()
                    }));
                fmt_layer.boxed()
            }
        }
        layer
    }
}
