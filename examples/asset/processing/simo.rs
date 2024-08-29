//! This example illustrates how to define custom `AssetLoader`s, `AssetTransformer`s, and `AssetSaver`s so that a single input asset can become multiple outputs.

use bevy::{
    asset::{
        embedded_asset,
        io::{Reader, Writer},
        processor::{LoadTransformAndMultiSave, LoadTransformAndSave, WriterContext},
        saver::{AssetSaver, SavedAsset},
        transformer::{AssetTransformer, TransformedAsset},
        AssetLoader, AsyncWriteExt, LoadContext,
    }, log::{Level, LogPlugin}, prelude::*, reflect::TypePath
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, io::ErrorKind, path::PathBuf};
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
                file_path: "examples/asset/processing/assets/simo".to_string(),
                processed_file_path: "examples/asset/processing/imported_assets/simo".to_string(),
                ..default()
            })
            .set(LogPlugin {
                filter: "".to_string(),
                level: Level::WARN,
                custom_layer: |_| None,
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
        app.init_asset::<SplitText>()
            .init_asset::<Text>()
            .init_asset::<NumericText>()
            .register_asset_loader(TextLoader)
            .register_asset_loader(NumericTextLoader)
            .register_asset_processor::<LoadTransformAndMultiSave<TextLoader, TextSplittingTransformer>>(
                LoadTransformAndMultiSave::new(TextSplittingTransformer, vec![
                    Box::new(NumericTextSaver),Box::new(LetterTextSaver)
                ]),
            )
            .set_default_asset_processor::<LoadTransformAndMultiSave<TextLoader, TextSplittingTransformer>>("txt");
    }
}

#[derive(Asset, TypePath, Debug)]
struct Text(String);

#[derive(Default)]
struct TextLoader;

impl AssetLoader for TextLoader {
    type Asset = Text;
    type Settings = ();
    type Error = std::io::Error;
    async fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Text, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let value = String::from_utf8(bytes).unwrap();
        Ok(Text(value))
    }

    fn extensions(&self) -> &[&str] {
        &["txt"]
    }
}

#[derive(Asset, TypePath, Debug)]
struct NumericText(String);

#[derive(Default)]
struct NumericTextLoader;

impl AssetLoader for NumericTextLoader {
    type Asset = NumericText;
    type Settings = ();
    type Error = std::io::Error;
    async fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<NumericText, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let value = String::from_utf8(bytes).unwrap();
        Ok(NumericText(value))
    }

    fn extensions(&self) -> &[&str] {
        &["numeric.txt"]
    }
}

#[derive(Asset, TypePath, Debug)]
struct SplitText {
    pub numbers: String,
    pub letters: String,
}

#[derive(Default)]
struct TextSplittingTransformer;

impl AssetTransformer for TextSplittingTransformer {
    type AssetInput = Text;
    type AssetOutput = SplitText;
    type Settings = ();
    type Error = Infallible;

    async fn transform<'a>(
        &'a self,
        mut asset: TransformedAsset<Self::AssetInput>,
        settings: &'a Self::Settings,
    ) -> Result<TransformedAsset<Self::AssetOutput>, Self::Error> {
        println!("incoming text: {:?}", asset.0);
        let mut split_text = SplitText {
            numbers: "".to_string(),
            letters: "".to_string(),
        };
        for line in asset.0.lines().into_iter() {
            if line.contains("1") {
                split_text.numbers += line;
            } else {
                split_text.letters += line;
            }
        }

        Ok(asset.replace_asset(split_text))
    }
}

struct NumericTextSaver;

impl AssetSaver for NumericTextSaver {
    type Asset = SplitText;
    type Settings = ();
    type OutputLoader = NumericTextLoader;
    type Error = std::io::Error;

    async fn save<'a>(
        &'a self,
        writer: &'a mut WriterContext<'_>,
        asset: SavedAsset<'a, Self::Asset>,
        _settings: Option<&'a Self::Settings>,
    ) -> Result<(), Self::Error> {
        let new_path = PathBuf::from("numbers.numeric.txt");
        writer
            .get_writer::<NumericTextLoader>(new_path.as_path(), Box::new(()))
            .await
            .map_err(|e| std::io::Error::new(ErrorKind::Other, "oh no!"))?
            .write_all(asset.numbers.as_bytes())
            .await
    }
}

struct LetterTextSaver;

impl AssetSaver for LetterTextSaver {
    type Asset = SplitText;
    type Settings = ();
    type OutputLoader = TextLoader;
    type Error = std::io::Error;

    async fn save<'a>(
        &'a self,
        writer: &'a mut WriterContext<'_>,
        asset: SavedAsset<'a, Self::Asset>,
        _settings: Option<&'a Self::Settings>,
    ) -> Result<(), Self::Error> {
        let old_path = writer.get_path();
        let path_str = old_path.to_str().unwrap();
        let mut parts: Vec<String> = path_str.split(".").map(|f| f.to_string()).collect();
        *parts.first_mut().unwrap() = "letters".to_string();
        let new_path_str = parts.join(".");
        let new_path = PathBuf::from(new_path_str);
        writer
            .get_writer::<TextLoader>(new_path.as_path(), Box::new(()))
            .await
            .map_err(|e| std::io::Error::new(ErrorKind::Other, "oh no!"))?
            .write_all(asset.letters.as_bytes())
            .await
    }
}

#[derive(Resource)]
struct TextAssets {
    a: Handle<NumericText>,
    b: Handle<Text>,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // This the final processed versions of `assets/a.cool.ron` and `assets/foo.c.cool.ron`
    // Check out their counterparts in `imported_assets` to see what the outputs look like.
    commands.insert_resource(TextAssets {
        a: assets.load("numbers.numeric.txt"),
        b: assets.load("letters.txt"),
    });
}

fn print_text(
    handles: Res<TextAssets>,
    texts: Res<Assets<Text>>,
    numeric_texts: Res<Assets<NumericText>>,
    mut asset_events: EventReader<AssetEvent<Text>>,
) {
    if !asset_events.is_empty() {
        // dbg!(&asset_events);

        // This prints the current values of the assets
        // Hot-reloading is supported, so try modifying the source assets (and their meta files)!
        println!("Current Values:");
        println!("  a: {:?}", numeric_texts.get(&handles.a));
        println!("  b: {:?}", texts.get(&handles.b));
        println!("(You can modify source assets and their .meta files to hot-reload changes!)");
        println!();
        asset_events.clear();
    }
}
