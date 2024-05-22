//! This example illustrates how to define custom `AssetLoader`s, `AssetTransformer`s, and `AssetSaver`s, how to configure them, and how to register asset processors.

use bevy::{
    asset::{
        embedded_asset,
        io::{Reader, Writer},
        processor::LoadTransformAndSave,
        saver::{AssetSaver, SavedAsset},
        transformer::{AssetTransformer, TransformedAsset},
        AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
    },
    prelude::*,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use thiserror::Error;

fn main() {
    App::new()
        // Using the "processed" mode will configure the AssetPlugin to use asset processing.
        // If you also enable the `asset_processor` cargo feature, this will run the AssetProcessor
        // in the background, run them through configured asset processors, and write the results to
        // the `imported_assets` folder. If you also enable the `file_watcher` cargo feature, changes to the
        // source assets will be detected and they will be reprocessed.
        //
        // The AssetProcessor will create `.meta` files automatically for assets in the `assets` folder,
        // which can then be used to configure how the asset will be processed.
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                mode: AssetMode::Processed,
                // This is just overriding the default paths to scope this to the correct example folder
                // You can generally skip this in your own projects
                file_path: "examples/asset/processing/assets".to_string(),
                processed_file_path: "examples/asset/processing/imported_assets/Default"
                    .to_string(),
                ..default()
            }),
            TextPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, print_text)
        .run();
}

/// This [`TextPlugin`] defines two assets types:
/// * [`CoolText`]: a custom RON text format that supports dependencies and embedded dependencies
/// * [`Text`]: a "normal" plain text file
///
/// It also defines an asset processor that will load [`CoolText`], resolve embedded dependencies, and write the resulting
/// output to a "normal" plain text file. When the processed asset is loaded, it is loaded as a Text (plaintext) asset.
/// This illustrates that when you process an asset, you can change its type! However you don't _need_ to change the type.
struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "examples/asset/processing/", "e.txt");
        app.init_asset::<CoolText>()
            .init_asset::<Text>()
            .register_asset_loader(CoolTextLoader)
            .register_asset_loader(TextLoader)
            .register_asset_processor::<LoadTransformAndSave<CoolTextLoader, CoolTextTransformer, CoolTextSaver>>(
                LoadTransformAndSave::new(CoolTextTransformer, CoolTextSaver),
            )
            .set_default_asset_processor::<LoadTransformAndSave<CoolTextLoader, CoolTextTransformer, CoolTextSaver>>("cool.ron");
    }
}

#[derive(Asset, TypePath, Debug)]
struct Text(String);

#[derive(Default)]
struct TextLoader;

#[derive(Clone, Default, Serialize, Deserialize)]
struct TextSettings {
    text_override: Option<String>,
}

impl AssetLoader for TextLoader {
    type Asset = Text;
    type Settings = TextSettings;
    type Error = std::io::Error;
    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        settings: &'a TextSettings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Text, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let value = if let Some(ref text) = settings.text_override {
            text.clone()
        } else {
            String::from_utf8(bytes).unwrap()
        };
        Ok(Text(value))
    }

    fn extensions(&self) -> &[&str] {
        &["txt"]
    }
}

#[derive(Serialize, Deserialize)]
struct CoolTextRon {
    text: String,
    dependencies: Vec<String>,
    embedded_dependencies: Vec<String>,
    dependencies_with_settings: Vec<(String, TextSettings)>,
}

#[derive(Asset, TypePath, Debug)]
struct CoolText {
    text: String,
    #[allow(unused)]
    dependencies: Vec<Handle<Text>>,
}

#[derive(Default)]
struct CoolTextLoader;

#[derive(Debug, Error)]
enum CoolTextLoaderError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    RonSpannedError(#[from] ron::error::SpannedError),
    #[error(transparent)]
    LoadDirectError(#[from] bevy::asset::LoadDirectError),
}

impl AssetLoader for CoolTextLoader {
    type Asset = CoolText;
    type Settings = ();
    type Error = CoolTextLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<CoolText, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let ron: CoolTextRon = ron::de::from_bytes(&bytes)?;
        let mut base_text = ron.text;
        for embedded in ron.embedded_dependencies {
            let loaded = load_context
                .loader()
                .direct()
                .load::<Text>(&embedded)
                .await?;
            base_text.push_str(&loaded.get().0);
        }
        for (path, settings_override) in ron.dependencies_with_settings {
            let loaded = load_context
                .loader()
                .with_settings(move |settings| {
                    *settings = settings_override.clone();
                })
                .direct()
                .load::<Text>(&path)
                .await?;
            base_text.push_str(&loaded.get().0);
        }
        Ok(CoolText {
            text: base_text,
            dependencies: ron
                .dependencies
                .iter()
                .map(|p| load_context.load(p))
                .collect(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["cool.ron"]
    }
}

#[derive(Default)]
struct CoolTextTransformer;

#[derive(Default, Serialize, Deserialize)]
struct CoolTextTransformerSettings {
    appended: String,
}

impl AssetTransformer for CoolTextTransformer {
    type AssetInput = CoolText;
    type AssetOutput = CoolText;
    type Settings = CoolTextTransformerSettings;
    type Error = Infallible;

    async fn transform<'a>(
        &'a self,
        mut asset: TransformedAsset<Self::AssetInput>,
        settings: &'a Self::Settings,
    ) -> Result<TransformedAsset<Self::AssetOutput>, Self::Error> {
        asset.text = format!("{}{}", asset.text, settings.appended);
        Ok(asset)
    }
}

struct CoolTextSaver;

impl AssetSaver for CoolTextSaver {
    type Asset = CoolText;
    type Settings = ();
    type OutputLoader = TextLoader;
    type Error = std::io::Error;

    async fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> Result<TextSettings, Self::Error> {
        writer.write_all(asset.text.as_bytes()).await?;
        Ok(TextSettings::default())
    }
}

#[derive(Resource)]
struct TextAssets {
    a: Handle<Text>,
    b: Handle<Text>,
    c: Handle<Text>,
    d: Handle<Text>,
    e: Handle<Text>,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // This the final processed versions of `assets/a.cool.ron` and `assets/foo.c.cool.ron`
    // Check out their counterparts in `imported_assets` to see what the outputs look like.
    commands.insert_resource(TextAssets {
        a: assets.load("a.cool.ron"),
        b: assets.load("foo/b.cool.ron"),
        c: assets.load("foo/c.cool.ron"),
        d: assets.load("d.cool.ron"),
        e: assets.load("embedded://asset_processing/e.txt"),
    });
}

fn print_text(
    handles: Res<TextAssets>,
    texts: Res<Assets<Text>>,
    mut asset_events: EventReader<AssetEvent<Text>>,
) {
    if !asset_events.is_empty() {
        // This prints the current values of the assets
        // Hot-reloading is supported, so try modifying the source assets (and their meta files)!
        println!("Current Values:");
        println!("  a: {:?}", texts.get(&handles.a));
        println!("  b: {:?}", texts.get(&handles.b));
        println!("  c: {:?}", texts.get(&handles.c));
        println!("  d: {:?}", texts.get(&handles.d));
        println!("  e: {:?}", texts.get(&handles.e));
        println!("(You can modify source assets and their .meta files to hot-reload changes!)");
        println!();
        asset_events.clear();
    }
}
