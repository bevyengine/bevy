#[cfg(target_os = "android")]
mod android_tracing;

pub mod prelude {
    pub use bevy_utils::tracing::{
        debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    };
}
pub use bevy_utils::tracing::{
    debug, debug_span, error, error_span, info, info_span, trace, trace_span, warn, warn_span,
    Level,
};

use bevy_app::{AppBuilder, Plugin};
#[cfg(feature = "tracing-chrome")]
use tracing_subscriber::fmt::{format::DefaultFields, FormattedFields};
use tracing_subscriber::{prelude::*, registry::Registry, EnvFilter};

/// Adds logging to Apps.
#[derive(Default)]
pub struct LogPlugin;

/// LogPlugin settings
pub struct LogSettings {
    /// Filters logs using the [EnvFilter] format
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
    fn build(&self, app: &mut AppBuilder) {
        let default_filter = {
            let settings = app.resources_mut().get_or_insert_with(LogSettings::default);
            format!("{},{}", settings.level, settings.filter)
        };

        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&default_filter))
            .unwrap();
        let subscriber = Registry::default().with(filter_layer);

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let fmt_layer = tracing_subscriber::fmt::Layer::default();
            let subscriber = subscriber.with(fmt_layer);
            #[cfg(feature = "tracing-chrome")]
            {
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
                app.resources_mut().insert_thread_local(guard);
                let subscriber = subscriber.with(chrome_layer);
                bevy_utils::tracing::subscriber::set_global_default(subscriber)
                    .expect("Could not set global default tracing subscriber. If you've already set up a tracing subscriber, please disable LogPlugin from Bevy's DefaultPlugins");
            }

            #[cfg(not(feature = "tracing-chrome"))]
            {
                bevy_utils::tracing::subscriber::set_global_default(subscriber)
                    .expect("Could not set global default tracing subscriber. If you've already set up a tracing subscriber, please disable LogPlugin from Bevy's DefaultPlugins");
            }
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
