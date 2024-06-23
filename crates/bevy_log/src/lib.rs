#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

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

use std::error::Error;
#[cfg(feature = "trace")]
use std::panic;

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

    #[doc(hidden)]
    pub use bevy_utils::{debug_once, error_once, info_once, once, trace_once, warn_once};
}

pub use bevy_utils::{
    debug_once, error_once, info_once, once, trace_once,
    tracing::{
        debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
        Level,
    },
    warn_once,
};
pub use tracing_subscriber;

use bevy_app::{App, Plugin};
use tracing_log::LogTracer;
#[cfg(feature = "tracing-chrome")]
use tracing_subscriber::fmt::{format::DefaultFields, FormattedFields};
use tracing_subscriber::{
    filter::{FromEnvError, ParseError},
    prelude::*,
    registry::Registry,
    EnvFilter, Layer,
};
#[cfg(feature = "tracing-chrome")]
use {bevy_ecs::system::Resource, bevy_utils::synccell::SyncCell};

/// Wrapper resource for `tracing-chrome`'s flush guard.
/// When the guard is dropped the chrome log is written to file.
#[cfg(feature = "tracing-chrome")]
#[allow(dead_code)]
#[derive(Resource)]
pub(crate) struct FlushGuard(SyncCell<tracing_chrome::FlushGuard>);

/// Adds logging to Apps. This plugin is part of the `DefaultPlugins`. Adding
/// this plugin will setup a collector appropriate to your target platform:
/// * Using [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber) by default,
///     logging to `stdout`.
/// * Using [`android_log-sys`](https://crates.io/crates/android_log-sys) on Android,
///     logging to Android logs.
/// * Using [`tracing-wasm`](https://crates.io/crates/tracing-wasm) in WASM, logging
///     to the browser console.
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
///             custom_layer: |_| None,
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

    /// Optionally add an extra [`Layer`] to the tracing subscriber
    ///
    /// This function is only called once, when the plugin is built.
    ///
    /// Because [`BoxedLayer`] takes a `dyn Layer`, `Vec<Layer>` is also an acceptable return value.
    ///
    /// Access to [`App`] is also provided to allow for communication between the [`Subscriber`]
    /// and the [`App`].
    ///
    /// Please see the `examples/log_layers.rs` for a complete example.
    pub custom_layer: fn(app: &mut App) -> Option<BoxedLayer>,
}

/// A boxed [`Layer`] that can be used with [`LogPlugin`].
pub type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

impl Default for LogPlugin {
    fn default() -> Self {
        Self {
            filter: "wgpu=error,naga=warn".to_string(),
            level: Level::INFO,
            custom_layer: |_| None,
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
                        eprintln!("LogPlugin failed to parse filter from env: {}", parse_err);
                    });

                Ok::<EnvFilter, FromEnvError>(EnvFilter::builder().parse_lossy(&default_filter))
            })
            .unwrap();
        let subscriber = subscriber.with(filter_layer);

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
                app.insert_resource(FlushGuard(SyncCell::new(guard)));
                chrome_layer
            };

            #[cfg(feature = "tracing-tracy")]
            let tracy_layer = tracing_tracy::TracyLayer::default();

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
            (true, true) => error!(
                "Could not set global logger and tracing subscriber as they are already set. Consider disabling LogPlugin."
            ),
            (true, false) => error!("Could not set global logger as it is already set. Consider disabling LogPlugin."),
            (false, true) => error!("Could not set global tracing subscriber as it is already set. Consider disabling LogPlugin."),
            (false, false) => (),
        }
    }
}
