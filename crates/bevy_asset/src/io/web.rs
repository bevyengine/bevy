use crate::io::{AssetReader, AssetReaderError, Reader};
use crate::io::{AssetSource, PathStream};
use crate::{AssetApp, AssetPlugin};
use alloc::{borrow::ToOwned, boxed::Box};
use bevy_app::{App, Plugin};
use bevy_tasks::ConditionalSendFuture;
use blocking::unblock;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Adds the `http` and `https` asset sources to the app.
///
/// NOTE: Make sure to add this plugin *before* `AssetPlugin` to properly register http asset sources.
///
/// WARNING: be careful about where your URLs are coming from! URLs can potentially be exploited by an
/// attacker to trigger vulnerabilities in our asset loaders, or DOS by downloading enormous files. We
/// are not aware of any such vulnerabilities at the moment, just be careful!
///
/// Any asset path that begins with `http` (when the `http` feature is enabled) or `https` (when the
/// `https` feature is enabled) will be loaded from the web via `fetch` (wasm) or `ureq` (native).
///
/// Example usage:
///
/// ```rust
/// # use bevy_app::{App, Startup};
/// # use bevy_ecs::prelude::{Commands, Res};
/// # use bevy_asset::web::{WebAssetPlugin, AssetServer};
/// # struct DefaultPlugins;
/// # impl DefaultPlugins { fn set(plugin: WebAssetPlugin) -> WebAssetPlugin { plugin } }
/// # use bevy_asset::web::AssetServer;
/// # #[derive(Asset, TypePath, Default)]
/// # struct Image;
/// # #[derive(Component)]
/// # struct Sprite;
/// # impl Sprite { fn from_image(_: Handle<Image>) -> Self { Sprite } }
/// # fn main() {
/// App::new()
///     .add_plugins(DefaultPlugins.set(WebAssetPlugin {
///         silence_startup_warning: true,
///     }))
/// #   .add_systems(Startup, setup).run();
/// # }
/// // ...
/// # fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
/// commands.spawn(Sprite::from_image(asset_server.load("https://example.com/favicon.png")));
/// # }
/// ```
///
/// By default, `ureq`'s HTTP compression is disabled. To enable gzip and brotli decompression, add
/// the following dependency and features to your Cargo.toml. This will improve bandwidth
/// utilization when its supported by the server.
///
/// ```toml
/// [target.'cfg(not(target_family = "wasm"))'.dev-dependencies]
/// ureq = { version = "3", default-features = false, features = ["gzip", "brotli"] }
/// ```
#[derive(Default)]
pub struct WebAssetPlugin {
    pub silence_startup_warning: bool,
}

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        if !self.silence_startup_warning {
            warn!("WebAssetPlugin is potentially insecure! Make sure to verify asset URLs are safe to load before loading them. \
            If you promise you know what you're doing, you can silence this warning by setting silence_startup_warning: true \
            in the WebAssetPlugin construction.");
        }
        if app.is_plugin_added::<AssetPlugin>() {
            warn!("WebAssetPlugin must be added before AssetPlugin for it to work!");
        }
        #[cfg(feature = "http")]
        app.register_asset_source(
            "http",
            AssetSource::build()
                .with_reader(move || Box::new(WebAssetReader::Http))
                .with_processed_reader(move || Box::new(WebAssetReader::Http)),
        );

        #[cfg(feature = "https")]
        app.register_asset_source(
            "https",
            AssetSource::build()
                .with_reader(move || Box::new(WebAssetReader::Https))
                .with_processed_reader(move || Box::new(WebAssetReader::Https)),
        );
    }
}

/// Asset reader that treats paths as urls to load assets from.
pub enum WebAssetReader {
    /// Unencrypted connections.
    Http,
    /// Use TLS for setting up connections.
    Https,
}

impl WebAssetReader {
    fn make_uri(&self, path: &Path) -> PathBuf {
        let prefix = match self {
            Self::Http => "http://",
            Self::Https => "https://",
        };
        PathBuf::from(prefix).join(path)
    }

    /// See [`crate::io::get_meta_path`]
    fn make_meta_uri(&self, path: &Path) -> PathBuf {
        let meta_path = crate::io::get_meta_path(path);
        self.make_uri(&meta_path)
    }
}

#[cfg(target_arch = "wasm32")]
async fn get<'a>(path: PathBuf) -> Result<Box<dyn Reader>, AssetReaderError> {
    use crate::io::wasm::HttpWasmAssetReader;

    HttpWasmAssetReader::new("")
        .fetch_bytes(path)
        .await
        .map(|r| Box::new(r) as Box<dyn Reader>)
}

#[cfg(not(target_arch = "wasm32"))]
async fn get(path: PathBuf) -> Result<Box<dyn Reader>, AssetReaderError> {
    use crate::io::VecReader;
    use alloc::{boxed::Box, vec::Vec};
    use bevy_platform::sync::LazyLock;
    use std::io::{self, BufReader, Read};

    let str_path = path.to_str().ok_or_else(|| {
        AssetReaderError::Io(
            io::Error::other(std::format!("non-utf8 path: {}", path.display())).into(),
        )
    })?;

    #[cfg(all(not(target_arch = "wasm32"), feature = "web_asset_cache"))]
    if let Some(data) = web_asset_cache::try_load_from_cache(str_path).await? {
        return Ok(Box::new(VecReader::new(data)));
    }
    use ureq::Agent;

    static AGENT: LazyLock<Agent> = LazyLock::new(|| Agent::config_builder().build().new_agent());

    let uri = str_path.to_owned();
    // Use [`unblock`] to run the http request on a separately spawned thread as to not block bevy's
    // async executor.
    let response = unblock(|| AGENT.get(uri).call()).await;

    match response {
        Ok(mut response) => {
            let mut reader = BufReader::new(response.body_mut().with_config().reader());

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;

            #[cfg(all(not(target_arch = "wasm32"), feature = "web_asset_cache"))]
            web_asset_cache::save_to_cache(str_path, &buffer).await?;

            Ok(Box::new(VecReader::new(buffer)))
        }
        // ureq considers all >=400 status codes as errors
        Err(ureq::Error::StatusCode(code)) => {
            if code == 404 {
                Err(AssetReaderError::NotFound(path))
            } else {
                Err(AssetReaderError::HttpError(code))
            }
        }
        Err(err) => Err(AssetReaderError::Io(
            io::Error::other(std::format!(
                "unexpected error while loading asset {}: {}",
                path.display(),
                err
            ))
            .into(),
        )),
    }
}

impl AssetReader for WebAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<dyn Reader>, AssetReaderError>> {
        get(self.make_uri(path))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader>, AssetReaderError> {
        let uri = self.make_meta_uri(path);
        get(uri).await
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(false)
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        Err(AssetReaderError::NotFound(self.make_uri(path)))
    }
}

/// A naive implementation of a cache for assets downloaded from the web that never invalidates.
/// `ureq` currently does not support caching, so this is a simple workaround.
/// It should eventually be replaced by `http-cache` or similar, see [tracking issue](https://github.com/06chaynes/http-cache/issues/91)
#[cfg(all(not(target_arch = "wasm32"), feature = "web_asset_cache"))]
mod web_asset_cache {
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::hash::{Hash, Hasher};
    use futures_lite::AsyncWriteExt;
    use std::collections::hash_map::DefaultHasher;
    use std::io;
    use std::path::PathBuf;

    use crate::io::Reader;

    const CACHE_DIR: &str = ".web-asset-cache";

    fn url_to_hash(url: &str) -> String {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        std::format!("{:x}", hasher.finish())
    }

    pub async fn try_load_from_cache(url: &str) -> Result<Option<Vec<u8>>, io::Error> {
        let filename = url_to_hash(url);
        let cache_path = PathBuf::from(CACHE_DIR).join(&filename);

        if cache_path.exists() {
            let mut file = async_fs::File::open(&cache_path).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;
            Ok(Some(buffer))
        } else {
            Ok(None)
        }
    }

    pub async fn save_to_cache(url: &str, data: &[u8]) -> Result<(), io::Error> {
        let filename = url_to_hash(url);
        let cache_path = PathBuf::from(CACHE_DIR).join(&filename);

        async_fs::create_dir_all(CACHE_DIR).await.ok();

        let mut cache_file = async_fs::File::create(&cache_path).await?;
        cache_file.write_all(data).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_http_uri() {
        assert_eq!(
            WebAssetReader::Http
                .make_uri(Path::new("example.com/favicon.png"))
                .to_str()
                .unwrap(),
            "http://example.com/favicon.png"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            WebAssetReader::Https
                .make_uri(Path::new("example.com/favicon.png"))
                .to_str()
                .unwrap(),
            "https://example.com/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            WebAssetReader::Http
                .make_meta_uri(Path::new("example.com/favicon.png"))
                .to_str()
                .unwrap(),
            "http://example.com/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            WebAssetReader::Https
                .make_meta_uri(Path::new("example.com/favicon.png"))
                .to_str()
                .unwrap(),
            "https://example.com/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_without_extension_meta_uri() {
        assert_eq!(
            WebAssetReader::Https
                .make_meta_uri(Path::new("example.com/favicon"))
                .to_str()
                .unwrap(),
            "https://example.com/favicon.meta"
        );
    }
}
