use bevy_app::{App, Plugin, ScheduleRunnerPlugin, Startup, Update};
use bevy_asset::{
    io::{Reader, Writer},
    processor::{AssetProcessor, LoadAndSave},
    saver::{AssetSaver, SavedAsset},
    Asset, AssetApp, AssetLoader, AssetPlugin, AssetServer, Assets, Handle, LoadContext,
};
use bevy_core::TaskPoolPlugin;
use bevy_ecs::prelude::*;
use bevy_log::{Level, LogPlugin};
use bevy_reflect::TypePath;
use bevy_utils::Duration;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

fn main() {
    App::new()
        .add_plugin(TaskPoolPlugin::default())
        .add_plugin(LogPlugin {
            level: Level::TRACE,
            ..Default::default()
        })
        .add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f32(2.0)))
        .add_plugin(AssetPlugin::processed_dev())
        .add_plugin(TextPlugin)
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
            .register_asset_loader(CoolTextLoader);

        if let Some(processor) = app.world.get_resource::<AssetProcessor>() {
            processor.register_processor::<LoadAndSave<CoolTextLoader, CoolTextSaver>>(
                CoolTextSaver.into(),
            );
        }
    }
}

#[derive(Asset, TypePath, Debug)]
struct Text(String);

#[derive(Default)]
struct TextLoader;

#[derive(Default, Serialize, Deserialize)]
struct TextSettings {
    blah: bool,
}

impl AssetLoader for TextLoader {
    type Asset = Text;
    type Settings = TextSettings;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a TextSettings,
        _load_context: &'a mut LoadContext,
    ) -> bevy_utils::BoxedFuture<'a, Result<Text, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let value = if settings.blah {
                "blah".to_string()
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
    ) -> bevy_utils::BoxedFuture<'a, Result<CoolText, anyhow::Error>> {
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
    ) -> bevy_utils::BoxedFuture<'a, Result<TextSettings, anyhow::Error>> {
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
