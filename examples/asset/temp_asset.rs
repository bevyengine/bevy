//! This example shows how to use the temporary asset source, `temp://`.
//! First, a [`TextAsset`] is created in-memory, then saved into the temporary asset source.
//! Once the save operation is completed, we load the asset just like any other file, and display its contents!

use bevy::{
    asset::{
        saver::{AssetSaver, ErasedAssetSaver},
        AssetPath, ErasedLoadedAsset, LoadedAsset, TempDirectory,
    },
    prelude::*,
    tasks::IoTaskPool,
};

use text_asset::{TextAsset, TextLoader, TextSaver};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_asset::<TextAsset>()
        .register_asset_loader(TextLoader)
        .add_systems(Startup, (save_temp_asset, setup_ui))
        .add_systems(Update, (wait_until_temp_saved, display_text))
        .run();
}

/// Attempt to save an asset to the temporary asset source.
fn save_temp_asset(assets: Res<AssetServer>, temp_directory: Res<TempDirectory>) {
    // This is the asset we will attempt to save.
    let my_text_asset =
        TextAsset("Hello World!\nPress the Down Arrow Key to Discard the Asset".to_owned());

    // To ensure the `Task` can outlive this function, we must provide owned versions
    // of the `AssetServer` and our desired path.
    let path = AssetPath::from("temp://message.txt").into_owned();
    let server = assets.clone();

    // We use Bevy's IoTaskPool to run the saving task asynchronously. This ensures
    // our application doesn't block during the (potentially lengthy!) saving process.
    // In this example, the asset is small so the blocking time will be short, but
    // that won't always be the case, especially for large assets.
    IoTaskPool::get()
        .spawn(async move {
            info!("Saving my asset...");
            save_asset(my_text_asset, path, server, TextSaver)
                .await
                .expect("Should've saved...");
            info!("...Saved!");
        })
        .detach();

    // You can check the logged path to see the temporary directory yourself. Note
    // that the directory will be deleted once this example quits.
    info!(
        "Temporary Assets will be saved in {:?}",
        temp_directory.path()
    );
}

/// Poll the save tasks until completion, and then start loading our temporary text asset.
fn wait_until_temp_saved(
    assets: Res<AssetServer>,
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        info!("Loading Asset...");
        commands.insert_resource(MyTempText {
            text: assets.load("temp://message.txt"),
        });
    }

    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        info!("Discarding Asset...");
        commands.remove_resource::<MyTempText>();
    }
}

/// Setup a basic UI to display our [`TextAsset`] once it's loaded.
fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((TextBundle::from_section(
        "Press the Up Arrow Key to Load The Asset...",
        default(),
    )
    .with_text_justify(JustifyText::Center)
    .with_style(Style {
        position_type: PositionType::Absolute,
        bottom: Val::Percent(50.),
        right: Val::Percent(50.),
        ..default()
    }),));
}

/// Once the [`TextAsset`] is loaded, update our display text to its contents.
fn display_text(
    mut query: Query<&mut Text>,
    my_text: Option<Res<MyTempText>>,
    texts: Res<Assets<TextAsset>>,
) {
    let message = my_text
        .as_ref()
        .and_then(|resource| texts.get(&resource.text))
        .map(|text| text.0.as_str())
        .unwrap_or("Press the Up Arrow Key to Load The Asset...");

    for mut text in query.iter_mut() {
        *text = Text::from_section(message, default());
    }
}

/// Save an [`Asset`] at the provided path. Returns [`None`] on failure.
async fn save_asset<A: Asset>(
    asset: A,
    path: AssetPath<'_>,
    server: AssetServer,
    saver: impl AssetSaver<Asset = A> + ErasedAssetSaver,
) -> Option<()> {
    let asset = ErasedLoadedAsset::from(LoadedAsset::from(asset));
    let source = server.get_source(path.source()).ok()?;
    let writer = source.writer().ok()?;

    let mut writer = writer.write(path.path()).await.ok()?;
    ErasedAssetSaver::save(&saver, &mut writer, &asset, &())
        .await
        .ok()?;

    Some(())
}

#[derive(Resource)]
struct MyTempText {
    text: Handle<TextAsset>,
}

mod text_asset {
    //! Putting the implementation of an asset loader and writer for a text asset in this module to avoid clutter.
    //! While this is required for this example to function, it isn't the focus.

    use bevy::{
        asset::{
            io::{Reader, Writer},
            saver::{AssetSaver, SavedAsset},
            AssetLoader, LoadContext,
        },
        prelude::*,
    };
    use futures_lite::{AsyncReadExt, AsyncWriteExt};

    #[derive(Asset, TypePath, Debug)]
    pub struct TextAsset(pub String);

    #[derive(Default)]
    pub struct TextLoader;

    impl AssetLoader for TextLoader {
        type Asset = TextAsset;
        type Settings = ();
        type Error = std::io::Error;
        async fn load<'a>(
            &'a self,
            reader: &'a mut Reader<'_>,
            _settings: &'a Self::Settings,
            _load_context: &'a mut LoadContext<'_>,
        ) -> Result<TextAsset, Self::Error> {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let value = String::from_utf8(bytes).unwrap();
            Ok(TextAsset(value))
        }

        fn extensions(&self) -> &[&str] {
            &["txt"]
        }
    }

    #[derive(Default)]
    pub struct TextSaver;

    impl AssetSaver for TextSaver {
        type Asset = TextAsset;
        type Settings = ();
        type OutputLoader = TextLoader;
        type Error = std::io::Error;

        async fn save<'a>(
            &'a self,
            writer: &'a mut Writer,
            asset: SavedAsset<'a, Self::Asset>,
            _settings: &'a Self::Settings,
        ) -> Result<(), Self::Error> {
            writer.write_all(asset.0.as_bytes()).await?;
            Ok(())
        }
    }
}
