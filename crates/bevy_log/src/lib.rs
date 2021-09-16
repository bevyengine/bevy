#[cfg(target_os = "android")]
mod android_tracing;

pub mod prelude {
    #[doc(hidden)]
    pub use bevy_utils::tracing::{
        debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    };
}
pub use bevy_utils::tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    Level,
};

use bevy_app::{App, Plugin};
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
/// You can configure this plugin using the resource [`LogSettings`].
/// ```no_run
/// # use bevy_internal::DefaultPlugins;
/// # use bevy_app::App;
/// # use bevy_log::LogSettings;
/// # use bevy_utils::tracing::Level;
/// fn main() {
///     App::new()
///         .insert_resource(LogSettings {
///             level: Level::DEBUG,
///             filter: "wgpu=error,bevy_render=info".to_string(),
///         })
///         .add_plugins(DefaultPlugins)
///         .run();
/// }
/// ```
///
/// Log level can also be changed using the `RUST_LOG` environment variable.
/// It has the same syntax has the field [`LogSettings::filter`], see [`EnvFilter`].
///
/// If you want to setup your own tracing collector, you should disable this
/// plugin from `DefaultPlugins` with [`App::add_plugins_with`]:
/// ```no_run
/// # use bevy_internal::DefaultPlugins;
/// # use bevy_app::App;
/// # use bevy_log::LogPlugin;
/// fn main() {
///     App::new()
///         .add_plugins_with(DefaultPlugins, |group| group.disable::<LogPlugin>())
///         .run();
/// }
/// ```
#[derive(Default)]
pub struct LogPlugin;

/// LogPlugin settings
pub struct LogSettings {
    /// Filters logs using the [`EnvFilter`] format
    pub filter: String,

    /// Filters out logs that are "less than" the given level.
    /// This can be further filtered using the `filter` setting.
    pub level: Level,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            filter: "wgpu=error".to_string(),
            level: Level::INFO,
        }
    }
}

impl Plugin for LogPlugin {
    fn build(&self, app: &mut App) {
        let default_filter = {
            let settings = app.world.get_resource_or_insert_with(LogSettings::default);
            format!("{},{}", settings.level, settings.filter)
        };

        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&default_filter))
            .unwrap();
        let subscriber = Registry::default().with(filter_layer);

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            #[cfg(feature = "tracing-chrome")]
            let chrome_layer = {
                let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
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
                app.world.insert_non_send(guard);
                chrome_layer
            };

            #[cfg(feature = "tracing-tracy")]
            let tracy_layer = tracing_tracy::TracyLayer::new();

            let fmt_layer = tracing_subscriber::fmt::Layer::default();
            let subscriber = subscriber.with(fmt_layer);

            #[cfg(feature = "tracing-chrome")]
            let subscriber = subscriber.with(chrome_layer);
            #[cfg(feature = "tracing-tracy")]
            let subscriber = subscriber.with(tracy_layer);

            bevy_utils::tracing::subscriber::set_global_default(subscriber)
                .expect("Could not set global default tracing subscriber. If you've already set up a tracing subscriber, please disable LogPlugin from Bevy's DefaultPlugins");
        }

        #[cfg(target_arch = "wasm32")]
        {
            console_error_panic_hook::set_once();
            let subscriber = subscriber.with(tracing_wasm::WASMLayer::new(
                tracing_wasm::WASMLayerConfig::default(),
            ));
            bevy_utils::tracing::subscriber::set_global_default(subscriber)
                .expect("Could not set global default tracing subscriber. If you've already set up a tracing subscriber, please disable LogPlugin from Bevy's DefaultPlugins");
        }

        #[cfg(target_os = "android")]
        {
            let subscriber = subscriber.with(android_tracing::AndroidLayer::default());
            bevy_utils::tracing::subscriber::set_global_default(subscriber)
                .expect("Could not set global default tracing subscriber. If you've already set up a tracing subscriber, please disable LogPlugin from Bevy's DefaultPlugins");
        }
    }
}
