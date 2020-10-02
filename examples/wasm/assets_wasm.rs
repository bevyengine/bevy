#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

use bevy::{asset::AssetLoader, prelude::*};
use std::path::PathBuf;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");
    }

    App::build()
        .add_default_plugins()
        .add_asset::<RustSourceCode>()
        .add_asset_loader::<RustSourceCode, RustSourceCodeLoader>()
        .add_startup_system(asset_system.system())
        .add_system(asset_events.system())
        .run();
}

fn asset_system(asset_server: Res<AssetServer>) {
    asset_server
        .load::<Handle<RustSourceCode>, _>(PathBuf::from("assets_wasm.rs"))
        .unwrap();
    log::info!("hello wasm");
}

#[derive(Debug)]
pub struct RustSourceCode(pub String);

#[derive(Default)]
pub struct RustSourceCodeLoader;
impl AssetLoader<RustSourceCode> for RustSourceCodeLoader {
    fn from_bytes(
        &self,
        _asset_path: &std::path::Path,
        bytes: Vec<u8>,
    ) -> Result<RustSourceCode, anyhow::Error> {
        Ok(RustSourceCode(String::from_utf8(bytes)?))
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
