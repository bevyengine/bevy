//! Adds the `http` and `https` asset sources to the app.
//!
//! See [`WebAssetPlugin`] for details.

use crate::io::{AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, Reader};
use crate::{AssetApp, AssetPlugin};
use alloc::boxed::Box;
use atomicow::CowArc;
use bevy_app::{App, Plugin};
use bevy_tasks::ConditionalSendFuture;
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
/// # use bevy_app::{App, Startup, TaskPoolPlugin};
/// # use bevy_ecs::prelude::{Commands, Component, Res};
/// # use bevy_asset::{Asset, AssetApp, AssetPlugin, AssetServer, Handle, io::web::WebAssetPlugin};
/// # use bevy_reflect::TypePath;
/// # struct DefaultPlugins;
/// # impl DefaultPlugins { fn set(&self, plugin: WebAssetPlugin) -> WebAssetPlugin { plugin } }
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
/// #   .add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
/// #   .init_asset::<Image>()
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
    /// Set this if you have seen the warning about URL safety enough times.
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
            AssetSourceBuilder::new(move || Box::new(WebAssetReader::Http))
                .with_processed_reader(move || Box::new(WebAssetReader::Http)),
        );

        #[cfg(feature = "https")]
        app.register_asset_source(
            "https",
            AssetSourceBuilder::new(move || Box::new(WebAssetReader::Https))
                .with_processed_reader(move || Box::new(WebAssetReader::Https)),
        );
    }
}

/// Asset reader that treats paths as URLs to load assets from.
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

    /// See [`io::get_meta_path`](`crate::io::get_meta_path`)
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
    use alloc::{borrow::ToOwned, boxed::Box, vec::Vec};
    use bevy_platform::sync::LazyLock;
    use blocking::unblock;
    use std::io::{self, Read};

    let str_path = path.to_str().ok_or_else(|| {
        AssetReaderError::Io(
            io::Error::other(std::format!("non-utf8 path: {}", path.display())).into(),
        )
    })?;

    #[cfg(target_os = "windows")]
    let str_path = &str_path.replace(std::path::MAIN_SEPARATOR, "/");

    // The cache is an optimization, a broken entry should not stop the asset from being downloaded.
    #[cfg(all(not(target_arch = "wasm32"), feature = "web_asset_cache"))]
    match web_asset_cache::try_load_from_cache(str_path).await {
        Ok(Some(data)) => return Ok(Box::new(VecReader::new(data))),
        Ok(None) => {}
        Err(err) => warn!("Failed to read {str_path} from the web asset cache: {err}"),
    }
    use ureq::tls::{RootCerts, TlsConfig};
    use ureq::Agent;

    static AGENT: LazyLock<Agent> = LazyLock::new(|| {
        Agent::config_builder()
            .tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .new_agent()
    });

    let uri = str_path.to_owned();
    // Use [`unblock`] to run the http request on a separately spawned thread as to not block bevy's
    // async executor. Reading the body blocks for as long as the download takes, so it has to
    // happen on that thread too.
    let response = unblock(|| {
        let mut response = AGENT.get(uri).call()?;
        let mut buffer = Vec::new();
        response
            .body_mut()
            .with_config()
            .reader()
            .read_to_end(&mut buffer)?;
        Ok::<_, ureq::Error>(buffer)
    })
    .await;

    match response {
        Ok(buffer) => {
            #[cfg(all(not(target_arch = "wasm32"), feature = "web_asset_cache"))]
            if let Err(err) = web_asset_cache::save_to_cache(str_path, &buffer).await {
                warn!("Failed to save {str_path} to the web asset cache: {err}");
            }

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
        path: CowArc<'a, Path>,
    ) -> impl ConditionalSendFuture<Output = Result<Box<dyn Reader>, AssetReaderError>> {
        get(self.make_uri(&path))
    }

    async fn read_meta<'a>(
        &'a self,
        path: CowArc<'a, Path>,
    ) -> Result<Box<dyn Reader>, AssetReaderError> {
        let uri = self.make_meta_uri(&path);
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
            #[cfg(feature = "multi_threaded")]
            let mut file = async_fs::File::open(&cache_path).await?;
            #[cfg(not(feature = "multi_threaded"))]
            let mut file =
                crate::io::file::sync_file_asset::FileReader(std::fs::File::open(&cache_path)?);
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

        // Entries are never invalidated, so a partially written one would be served forever.
        // Write to a temporary file and only move it into place once it is complete.
        let temp_path = cache_path.with_extension("tmp");
        let written = async {
            let mut cache_file = async_fs::File::create(&temp_path).await?;
            cache_file.write_all(data).await?;
            cache_file.close().await?;
            async_fs::rename(&temp_path, &cache_path).await
        }
        .await;

        if written.is_err() {
            async_fs::remove_file(&temp_path).await.ok();
        }
        written
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

    /// Serves `response` once on a local port and returns the port.
    #[cfg(all(
        feature = "http",
        not(feature = "web_asset_cache"),
        not(target_arch = "wasm32")
    ))]
    fn serve_once(response: &'static [u8]) -> u16 {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = alloc::vec::Vec::new();
            let mut chunk = [0; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let read = stream.read(&mut chunk).unwrap();
                request.extend_from_slice(&chunk[..read]);
            }
            stream.write_all(response).unwrap();
        });
        port
    }

    #[cfg(all(
        feature = "http",
        not(feature = "web_asset_cache"),
        not(target_arch = "wasm32")
    ))]
    async fn read_from_local_server(
        response: &'static [u8],
    ) -> Result<alloc::vec::Vec<u8>, AssetReaderError> {
        let port = serve_once(response);
        let path = std::format!("127.0.0.1:{port}/asset.txt");
        let mut reader = WebAssetReader::Http
            .read(CowArc::Borrowed(Path::new(&path)))
            .await?;
        let mut bytes = alloc::vec::Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(bytes)
    }

    #[cfg(all(
        feature = "http",
        not(feature = "web_asset_cache"),
        not(target_arch = "wasm32")
    ))]
    #[test]
    fn reads_the_response_body() {
        let bytes = bevy_tasks::block_on(read_from_local_server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        ))
        .unwrap();
        assert_eq!(bytes, b"hello");
    }

    #[cfg(all(
        feature = "http",
        not(feature = "web_asset_cache"),
        not(target_arch = "wasm32")
    ))]
    #[test]
    fn a_missing_asset_is_not_found() {
        let result = bevy_tasks::block_on(read_from_local_server(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ));
        assert!(matches!(result, Err(AssetReaderError::NotFound(_))));
    }

    #[cfg(all(
        feature = "http",
        not(feature = "web_asset_cache"),
        not(target_arch = "wasm32")
    ))]
    #[test]
    fn a_body_cut_short_is_an_error() {
        let result = bevy_tasks::block_on(read_from_local_server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 50\r\nConnection: close\r\n\r\nhello",
        ));
        assert!(matches!(result, Err(AssetReaderError::Io(_))));
    }
}
