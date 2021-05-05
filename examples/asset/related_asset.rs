use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CustomAsset {
    pub texture: String,
}

#[derive(Debug, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct RelatedAsset {
    pub texture: Option<Handle<Texture>>,
}

#[derive(Default)]
pub struct RelatedAssetLoader;

impl AssetLoader for RelatedAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let custom_asset = ron::de::from_bytes::<CustomAsset>(bytes)?;
            let asset_loader = load_context.asset_server();
            let texture = asset_loader
                .is_file(&custom_asset.texture)
                .then(|| asset_loader.load(custom_asset.texture.as_str()));
            load_context.set_default_asset(LoadedAsset::new(RelatedAsset { texture }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["custom"]
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .init_resource::<State>()
        .add_asset::<RelatedAsset>()
        .init_asset_loader::<RelatedAssetLoader>()
        .add_startup_system(setup.system())
        .add_system(spawn_sprite_on_load.system())
        .run();
}

#[derive(Default)]
struct State {
    handle: Handle<RelatedAsset>,
}

fn setup(mut state: ResMut<State>, asset_server: Res<AssetServer>) {
    state.handle = asset_server.load("data/related.custom");
}

fn spawn_sprite_on_load(
    state: ResMut<State>,
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut related_assets: ResMut<Assets<RelatedAsset>>,
) {
    let related_asset = related_assets.remove(&state.handle);
    if related_asset.is_none() {
        return;
    }
    let related_asset = related_asset.unwrap();
    if related_asset.texture.is_none() {
        return;
    }

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(related_asset.texture.unwrap().into()),
        ..Default::default()
    });
}
