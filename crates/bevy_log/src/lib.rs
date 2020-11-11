pub mod prelude {
    pub use bevy_app::{AppBuilder, Plugin};
    pub use bevy_utils::tracing::*;
}
pub use bevy_app::{AppBuilder, Plugin};
pub use bevy_utils::tracing::*;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

/// Adds logging to Apps.
#[derive(Default)]
pub struct LogPlugin;

impl Plugin for LogPlugin {
    fn build(&self, _app: &mut AppBuilder) {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        setup_default(_app);
        #[cfg(target_arch = "wasm32")]
        setup_wasm();
        #[cfg(target_arch = "android")]
        setup_android();
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn setup_default(_app: &mut AppBuilder) {
    let fmt_layer = fmt::Layer::default();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,wgpu=warn"))
        .unwrap();

    let subscriber = Registry::default().with(filter_layer).with(fmt_layer);
    #[cfg(feature = "tracing-chrome")]
    {
        let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new().build();
        _app.resources_mut().insert_thread_local(guard);
        let subscriber = subscriber.with(chrome_layer);
        bevy_utils::tracing::subscriber::set_global_default(subscriber)
            .expect("Could not set global default tracing subscriber");
    }

    #[cfg(not(feature = "tracing-chrome"))]
    {
        bevy_utils::tracing::subscriber::set_global_default(subscriber)
            .expect("Could not set global default tracing subscriber");
    }
}

#[cfg(target_arch = "wasm32")]
fn setup_wasm() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

#[cfg(target_os = "android")]
fn setup_android() {}
