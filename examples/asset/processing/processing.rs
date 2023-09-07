//! This example illustrates how to define custom `AssetLoader`s and `AssetSaver`s, how to configure them, and how to register asset processors.

use bevy::{
    asset::{
        io::{AssetProviders, Reader, Writer},
        processor::LoadAndSave,
        saver::{AssetSaver, SavedAsset},
        AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
    },
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};
use serde::{Deserialize, Serialize};

fn main() {
    App::new()
        .insert_resource(
            // This is just overriding the default paths to scope this to the correct example folder
            // You can generally skip this in your own projects
            AssetProviders::default()
                .with_default_file_source("examples/asset/processing/assets".to_string())
                .with_default_file_destination(
                    "examples/asset/processing/imported_assets".to_string(),
                ),
        )
        // Enabling `processed_dev` will configure the AssetPlugin to use asset processing.
        // This will run the AssetProcessor in the background, which will listen for changes to
        // the `assets` folder, run them through configured asset processors, and write the results
        // to the `imported_assets` folder.
        //
        // The AssetProcessor will create `.meta` files automatically for assets in the `assets` folder,
        // which can then be used to configure how the asset will be processed.
        .add_plugins((DefaultPlugins.set(AssetPlugin::processed_dev()), TextPlugin))
        // This is what a deployed app should use
        // .add_plugins((DefaultPlugins.set(AssetPlugin::processed()), TextPlugin))
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
pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<CoolText>()
            .init_asset::<Text>()
            .register_asset_loader(CoolTextLoader)
            .register_asset_loader(TextLoader)
            .register_asset_processor::<LoadAndSave<CoolTextLoader, CoolTextSaver>>(
                LoadAndSave::from(CoolTextSaver),
            )
            .set_default_asset_processor::<LoadAndSave<CoolTextLoader, CoolTextSaver>>("cool.ron");
    }
}

#[derive(Asset, TypePath, Debug)]
struct Text(String);

#[derive(Default)]
struct TextLoader;

#[derive(Default, Serialize, Deserialize)]
struct TextSettings {
    text_override: Option<String>,
}

impl AssetLoader for TextLoader {
    type Asset = Text;
    type Settings = TextSettings;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a TextSettings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Text, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let value = if let Some(ref text) = settings.text_override {
                text.clone()
            } else {
                String::from_utf8(bytes).unwrap()
            };
            Ok(Text(value))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["txt"]
    }
}

#[derive(Serialize, Deserialize)]
pub struct CoolTextRon {
    text: String,
    dependencies: Vec<String>,
    embedded_dependencies: Vec<String>,
}

#[derive(Asset, TypePath, Debug)]
pub struct CoolText {
    text: String,
    #[allow(unused)]
    dependencies: Vec<Handle<Text>>,
}

#[derive(Default)]
struct CoolTextLoader;

impl AssetLoader for CoolTextLoader {
    type Asset = CoolText;

    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<CoolText, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let ron: CoolTextRon = ron::de::from_bytes(&bytes)?;
            let mut base_text = ron.text;
            for embedded in ron.embedded_dependencies {
                let loaded = load_context.load_direct(&embedded).await?;
                let text = loaded.get::<Text>().unwrap();
                base_text.push_str(&text.0);
            }
            Ok(CoolText {
                text: base_text,
                dependencies: ron
                    .dependencies
                    .iter()
                    .map(|p| load_context.load(p))
                    .collect(),
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["cool.ron"]
    }
}

struct CoolTextSaver;

#[derive(Default, Serialize, Deserialize)]
pub struct CoolTextSaverSettings {
    appended: String,
}

impl AssetSaver for CoolTextSaver {
    type Asset = CoolText;
    type Settings = CoolTextSaverSettings;
    type OutputLoader = TextLoader;

    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        settings: &'a Self::Settings,
    ) -> BoxedFuture<'a, Result<TextSettings, anyhow::Error>> {
        Box::pin(async move {
            let text = format!("{}{}", asset.text.clone(), settings.appended);
            writer.write_all(text.as_bytes()).await?;
            Ok(TextSettings::default())
        })
    }
}

#[derive(Resource)]
struct TextAssets {
    a: Handle<Text>,
    b: Handle<Text>,
    c: Handle<Text>,
    d: Handle<Text>,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // This the final processed versions of `assets/a.cool.ron` and `assets/foo.c.cool.ron`
    // Check out their counterparts in `imported_assets` to see what the outputs look like.
    commands.insert_resource(TextAssets {
        a: assets.load("a.cool.ron"),
        b: assets.load("foo/b.cool.ron"),
        c: assets.load("foo/c.cool.ron"),
        d: assets.load("d.cool.ron"),
    });
}

fn print_text(handles: Res<TextAssets>, texts: Res<Assets<Text>>) {
    // This prints the current values of the assets
    // Hot-reloading is supported, so try modifying the source assets (and their meta files)!
    println!("Current Values:");
    println!("  a: {:?}", texts.get(&handles.a));
    println!("  b: {:?}", texts.get(&handles.b));
    println!("  c: {:?}", texts.get(&handles.c));
    println!("  d: {:?}", texts.get(&handles.d));
    println!("(You can modify source assets and their .meta files to hot-reload changes!)");
    println!();
}
