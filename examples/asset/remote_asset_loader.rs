//! Implements a remote asset loader.
//! An [`AssetReader`] is what the asset server uses to read the raw bytes of assets.
//! This example also showcases how to use [`AssetWriter`].
//! Note that it won't work on wasm or Android because these platforms lack a default
//! [`AssetWriter`] implementation for now, but it's possible to implement your own.

use bevy::{
    asset::io::{
        AssetReader, AssetReaderError, AssetSource, AssetSourceId, AssetWriter, AssetWriterError,
        PathStream, Reader,
    },
    log::LogPlugin,
    prelude::*,
    utils::BoxedFuture,
};
use futures_lite::AsyncRead;
use std::{path::Path, sync::Arc};

/// A remote asset loader implementation that will try to read asset local and, if not found,
/// download it remotely from a CDN.
struct RemoteAssetLoader {
    cdn_prefix: String,
    reader: Box<dyn AssetReader>,
    writer: Box<dyn AssetWriter>,
}

impl RemoteAssetLoader {
    fn new() -> Self {
        // Create the root directory if it doesn't exists on local disk.
        let create_root_dir = true;
        let writer = AssetSource::get_default_writer("assets".to_string())(create_root_dir)
            .expect("Current platform doesn't support asset writing.");
        let reader = AssetSource::get_default_reader("assets".to_string())();

        // This is the CDN which will be downloaded the font when missing localy.
        let cdn_prefix =
            "https://github.com/google/fonts/raw/main/ofl/protestrevolution/".to_string();

        RemoteAssetLoader {
            reader,
            writer,
            cdn_prefix,
        }
    }

    /// Remote download the asset from CDN.
    async fn read_remote<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<dyn AsyncRead + Send + Sync + Unpin + 'a>, AssetReaderError> {
        // Only the asset name is needed, not the folder.
        let asset_name = path.file_name().unwrap().to_str().unwrap();
        let mut url = self.cdn_prefix.clone();
        url.push_str(asset_name);

        // A simple GET request is used, but you could set custom headers, auth and so on.
        let request = ehttp::Request::get(url);

        let body = match ehttp::fetch_async(request).await {
            Ok(response) => {
                // Since this is an example, only check for 200 status, but in a real world use
                // it would be wise to check for others 2xx or 3xx status.
                if response.status != 200 {
                    return Err(AssetReaderError::HttpError(response.status));
                }

                response.bytes
            }
            Err(error) => {
                warn!("Failed to read remote asset: {error}");
                return Err(AssetReaderError::HttpError(500));
            }
        };

        // Try to save the downloaded asset on disk.
        if let Err(AssetWriterError::Io(error)) = self.writer.write_bytes(path, &body).await {
            return Err(AssetReaderError::Io(Arc::new(error)));
        }

        info!("Successfully downloaded asset {path:?} from CDN. Loading it now.");

        // Try to read again the asset, since its saved on disk now.
        self.reader.read(path).await
    }
}

impl AssetReader for RemoteAssetLoader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            // First check if the default reader will find the asset local. If it doesn't find
            // then try to read from CDN.
            match self.reader.read(path).await {
                Ok(reader) => Ok(reader),
                Err(error) => match error {
                    AssetReaderError::NotFound(_) => {
                        info!("Asset {path:?} not found on local disk. Downloading from CDN.");
                        self.read_remote(path).await
                    }
                    _ => Err(error),
                },
            }
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        // Just forward the call to default reader.
        self.reader.read_meta(path)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        // Just forward the call to default reader.
        self.reader.read_directory(path)
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        // Just forward the call to default reader.
        self.reader.is_directory(path)
    }
}

/// A plugins that registers our new asset loader
struct RemoteAssetLoaderPlugin;

impl Plugin for RemoteAssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSource::build().with_reader(|| Box::new(RemoteAssetLoader::new())),
        );
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        RemoteAssetLoaderPlugin,
        DefaultPlugins.set(LogPlugin {
            filter: "remote_asset_loader=info".to_string(),
            ..Default::default()
        }),
    ))
    .add_systems(Startup, setup)
    .run();

    // Just a cleanup to remove the downloaded font, so the example will be able to download
    // next time.
    let _ = std::fs::remove_file("assets/fonts/ProtestRevolution-Regular.ttf");
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(TextBundle::from_section(
        "This font was loaded\nfrom a CDN",
        TextStyle {
            // If this font exists on local disk, it will be used, else it will be downloaded
            // from our CDN and then loaded.
            font: asset_server.load("fonts/ProtestRevolution-Regular.ttf"),
            font_size: 50.0,
            ..default()
        },
    ));

    commands.spawn(
        TextBundle::from_section(
            "This font was lodaded\nfrom local disk",
            TextStyle {
                // This font already exists, so nothing will be downloaded.
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 50.0,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        }),
    );
}
