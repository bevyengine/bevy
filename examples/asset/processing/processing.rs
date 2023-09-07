use bevy::{
    asset::{
        io::{Reader, Writer},
        processor::LoadAndSave,
        saver::{AssetSaver, SavedAsset},
        AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
    },
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};
use bevy_internal::asset::io::AssetProviders;
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
        .add_plugins((DefaultPlugins.set(AssetPlugin::processed_dev()), TextPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, print_text)
        .run();
}

pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Text>()
            .init_asset::<CoolText>()
            .register_asset_loader(TextLoader)
            .register_asset_loader(CoolTextLoader)
            .register_asset_processor::<LoadAndSave<CoolTextLoader, CoolTextSaver>>(
                CoolTextSaver.into(),
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
    c: Handle<Text>,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(TextAssets {
        a: assets.load("a.cool.ron"),
        c: assets.load("foo/c.cool.ron"),
    });
}

fn print_text(handles: Res<TextAssets>, texts: Res<Assets<Text>>) {
    println!("a {:?}", texts.get(&handles.a));
    println!("c {:?}", texts.get(&handles.c));
}
