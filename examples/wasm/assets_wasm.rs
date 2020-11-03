#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadContext, LoadedAsset},
    prelude::*,
    type_registry::TypeUuid,
    utils::BoxedFuture,
};

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");
    }

    App::build()
        .add_resource(AssetServerSettings {
            asset_folder: "/".to_string(),
        })
        .add_plugin_group(DefaultPlugins)
        .add_asset::<RustSourceCode>()
        .init_asset_loader::<RustSourceCodeLoader>()
        .add_startup_system(load_asset.system())
        .add_system(print_asset.system())
        .run();
}

struct State {
    handle: Handle<RustSourceCode>,
    printed: bool,
}

fn load_asset(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(State {
        handle: asset_server.load("assets_wasm.rs"),
        printed: false,
    });
}

fn print_asset(mut state: ResMut<State>, rust_sources: Res<Assets<RustSourceCode>>) {
    if state.printed {
        return;
    }

    if let Some(code) = rust_sources.get(&state.handle) {
        log::info!("code: {}", code.0);
        state.printed = true;
    }
}

#[derive(Debug, TypeUuid)]
#[uuid = "1c3445ab-97d3-449c-ab35-16ba30e4c29d"]
pub struct RustSourceCode(pub String);

#[derive(Default)]
pub struct RustSourceCodeLoader;

impl AssetLoader for RustSourceCodeLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            load_context.set_default_asset(LoadedAsset::new(RustSourceCode(String::from_utf8(
                bytes.into(),
            )?)));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXT: &[&str] = &["rs"];
        EXT
    }
}
