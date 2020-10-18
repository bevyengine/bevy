#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

use bevy::{asset::AssetLoader, prelude::*, type_registry::TypeUuid};
use bevy_asset::{LoadContext, LoadedAsset};

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");
    }

    App::build()
        .add_default_plugins()
        .add_asset::<RustSourceCode>()
        .init_asset_loader::<RustSourceCodeLoader>()
        .add_startup_system(asset_system.system())
        .add_system(asset_events.system())
        .run();
}

fn asset_system(asset_server: Res<AssetServer>) {
    asset_server.load::<RustSourceCode, _>("assets_wasm.rs");
    log::info!("hello wasm");
}

#[derive(Debug, TypeUuid)]
#[uuid = "1c3445ab-97d3-449c-ab35-16ba30e4c29d"]
pub struct RustSourceCode(pub String);

#[derive(Default)]
pub struct RustSourceCodeLoader;

impl AssetLoader for RustSourceCodeLoader {
    fn load(&self, bytes: &[u8], load_context: &mut LoadContext) -> Result<(), anyhow::Error> {
        load_context.set_default_asset(LoadedAsset::new(RustSourceCode(String::from_utf8(
            bytes.into(),
        )?)));
        Ok(())
    }

    fn extensions(&self) -> &[&str] {
        static EXT: &[&str] = &["rs"];
        EXT
    }
}

#[derive(Default)]
pub struct AssetEventsState {
    reader: EventReader<AssetEvent<RustSourceCode>>,
}

pub fn asset_events(
    mut state: Local<AssetEventsState>,
    rust_sources: Res<Assets<RustSourceCode>>,
    events: Res<Events<AssetEvent<RustSourceCode>>>,
) {
    for event in state.reader.iter(&events) {
        match event {
            AssetEvent::Created { handle } => {
                if let Some(code) = rust_sources.get(handle) {
                    log::info!("code: {}", code.0);
                }
            }
            _ => continue,
        };
    }
}
