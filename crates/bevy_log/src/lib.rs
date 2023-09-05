#![allow(clippy::type_complexity)]
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

#[cfg(feature = "trace")]
use std::panic;

use std::path::PathBuf;

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
}

use bevy_ecs::system::Resource;
pub use bevy_utils::tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    Level,
};

use bevy_app::{App, Plugin};
use tracing_log::LogTracer;
#[cfg(feature = "tracing-chrome")]
use tracing_subscriber::fmt::{format::DefaultFields, FormattedFields};
use tracing_subscriber::{prelude::*, registry::Registry, EnvFilter};

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
///             file_appender_settings: None
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

    /// Configure file logging
    ///
    /// ## Platform-specific
    ///
    /// **`WASM`** does not support logging to a file.
    pub file_appender_settings: Option<FileAppenderSettings>,
}

impl Default for LogPlugin {
    fn default() -> Self {
        Self {
            filter: "wgpu=error,naga=warn".to_string(),
            level: Level::INFO,
            file_appender_settings: None,
        }
    }
}

/// Enum to control how often a new log file will be created
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rolling {
    /// Creates a new file every minute and appends the date to the file name
    /// Date format: YYYY-MM-DD-HH-mm
    Minutely,
    /// Creates a new file every hour and appends the date to the file name
    /// Date format: YYYY-MM-DD-HH
    Hourly,
    /// Creates a new file every day and appends the date to the file name
    /// Date format: YYYY-MM-DD
    Daily,
    /// Never creates a new file
    Never,
}

impl From<Rolling> for tracing_appender::rolling::Rotation {
    fn from(val: Rolling) -> Self {
        match val {
            Rolling::Minutely => tracing_appender::rolling::Rotation::MINUTELY,
            Rolling::Hourly => tracing_appender::rolling::Rotation::HOURLY,
            Rolling::Daily => tracing_appender::rolling::Rotation::DAILY,
            Rolling::Never => tracing_appender::rolling::Rotation::NEVER,
        }
    }
}

#[derive(Resource)]
struct FileAppenderWorkerGuard(tracing_appender::non_blocking::WorkerGuard);

/// Settings to control how to log to a file
#[derive(Debug, Clone)]
pub struct FileAppenderSettings {
    /// Controls how often a new file will be created
    pub rolling: Rolling,
    /// The path of the directory where the log files will be added
    ///
    /// Defaults to the local directory
    pub path: PathBuf,
    /// The prefix added when creating a file
    pub prefix: String,
    /// When this is enabled, a panic hook will be used and any panic will be logged as an error
    pub use_panic_hook: bool,
}

impl Default for FileAppenderSettings {
    fn default() -> Self {
        Self {
            rolling: Rolling::Never,
            path: PathBuf::from("."),
            prefix: String::from("log"),
            use_panic_hook: true,
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
                println!("{}", tracing_error::SpanTrace::capture());
                old_handler(infos);
            }));
        }

        let finished_subscriber;
        let default_filter = { format!("{},{}", self.level, self.filter) };
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&default_filter))
            .unwrap();
        let subscriber = Registry::default().with(filter_layer);

        #[cfg(feature = "trace")]
        let subscriber = subscriber.with(tracing_error::ErrorLayer::default());

        #[cfg(not(target_arch = "wasm32"))]
        {
            #[cfg(not(target_os = "android"))]
            let subscriber = {
                #[cfg(feature = "tracing-chrome")]
                let chrome_layer = {
                    let mut layer = tracing_chrome::ChromeLayerBuilder::new();
                    if let Ok(path) = std::env::var("TRACE_CHROME") {
                        layer = layer.file(path);
                    }
                    let (chrome_layer, guard) = layer
                        .name_fn(Box::new(|event_or_span| match event_or_span {
                            tracing_chrome::EventOrSpan::Event(event) => {
                                event.metadata().name().into()
                            }
                            tracing_chrome::EventOrSpan::Span(span) => {
                                if let Some(fields) =
                                    span.extensions().get::<FormattedFields<DefaultFields>>()
                                {
                                    format!(
                                        "{}: {}",
                                        span.metadata().name(),
                                        fields.fields.as_str()
                                    )
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

                let fmt_layer =
                    tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr);

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
                subscriber
            };

            let file_appender_layer = if let Some(settings) = &self.file_appender_settings {
                if settings.use_panic_hook {
                    let old_handler = std::panic::take_hook();
                    std::panic::set_hook(Box::new(|panic_info| {
                        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                            error!("panic occurred: {s:?}");
                        } else {
                            error!("panic occurred");
                        }
                        old_handler(panic_info);
                    }));
                }

                if settings.rolling == Rolling::Never && settings.prefix.is_empty() {
                    panic!("Using the Rolling::Never variant with no prefix will result in an empty filename which is invalid");
                }
                let file_appender = tracing_appender::rolling::RollingFileAppender::new(
                    settings.rolling.into(),
                    &settings.path,
                    &settings.prefix,
                );

                let (non_blocking, worker_guard) = tracing_appender::non_blocking(file_appender);
                // WARN We need to keep this somewhere so it doesn't get dropped.
                // If it gets dropped then it will silently stop writing to the file
                app.insert_resource(FileAppenderWorkerGuard(worker_guard));

                let file_fmt_layer = tracing_subscriber::fmt::Layer::default()
                    .with_ansi(false)
                    .with_writer(non_blocking);
                Some(file_fmt_layer)
            } else {
                None
            };
            let subscriber = subscriber.with(file_appender_layer);

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
