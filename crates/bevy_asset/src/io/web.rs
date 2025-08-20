use crate::io::{AssetReader, AssetReaderError, Reader};
use crate::io::{AssetSource, PathStream};
use crate::{AssetApp, AssetServer};
use alloc::{borrow::ToOwned, boxed::Box, vec, vec::Vec};
use bevy_app::{App, Plugin};
use bevy_tasks::ConditionalSendFuture;
use blocking::unblock;
use core::str::FromStr;
use std::path::{Path, PathBuf};
use tracing::error;
use url::Url;

// TODO: taken from https://doc.rust-lang.org/stable/std/path/struct.Path.html#method.normalize_lexically
// replace when https://github.com/rust-lang/rust/issues/134694 is stable
struct NormalizeError;
fn normalize_path_lexically(path: &Path) -> Result<PathBuf, NormalizeError> {
    use std::path::Component;
    let mut lexical = PathBuf::new();
    let mut iter = path.components().peekable();

    // Find the root, if any, and add it to the lexical path.
    // Here we treat the Windows path "C:\" as a single "root" even though
    // `components` splits it into two: (Prefix, RootDir).
    let root = match iter.peek() {
        Some(Component::ParentDir) => return Err(NormalizeError),
        Some(p @ Component::RootDir) | Some(p @ Component::CurDir) => {
            lexical.push(p);
            iter.next();
            lexical.as_os_str().len()
        }
        Some(Component::Prefix(prefix)) => {
            lexical.push(prefix.as_os_str());
            iter.next();
            if let Some(p @ Component::RootDir) = iter.peek() {
                lexical.push(p);
                iter.next();
            }
            lexical.as_os_str().len()
        }
        None => return Ok(PathBuf::new()),
        Some(Component::Normal(_)) => 0,
    };

    for component in iter {
        match component {
            Component::RootDir => unreachable!(),
            Component::Prefix(_) => return Err(NormalizeError),
            Component::CurDir => continue,
            Component::ParentDir => {
                // It's an error if ParentDir causes us to go above the "root".
                if lexical.as_os_str().len() == root {
                    return Err(NormalizeError);
                }
                lexical.pop();
            }
            Component::Normal(path) => lexical.push(path),
        }
    }
    Ok(lexical)
}

/// Adds the `http` and `https` asset sources to the app.
///
/// NOTE: Make sure to add this plugin *before* `AssetPlugin` to properly register http asset sources.
///
/// Any asset path that begins with `http` (when the `http` feature is enabled) or `https` (when the
/// `https` feature is enabled) will be loaded from the web via `fetch` (wasm) or `ureq` (native).
///
/// You must provide valid URLs to the `WebAssetPlugin` constructor, either [`Self::allowed_url`] or [`Self::allowed_urls`].
/// Only assets available on those URLs can be used.
///
/// Example usage:
///
/// ```rust
/// # use bevy_app::{App, Startup, Plugin};
/// # use bevy_ecs::prelude::{Commands, Res, Component};
/// # use bevy_asset::io::web::WebAssetPlugin;
/// # use bevy_asset::{AssetServer, Asset, Handle};
/// # use bevy_reflect::TypePath;
/// # struct DefaultPlugins;
/// # impl Plugin for DefaultPlugins { fn build(&self, _: &mut App) { } }
/// # #[derive(Asset, TypePath, Default)]
/// # struct Image;
/// # #[derive(Component)]
/// # struct Sprite;
/// # impl Sprite { fn from_image(_: Handle<Image>) -> Self { Sprite } }
/// # fn main() {
/// App::new()
///     .add_plugins((
///         WebAssetPlugin::allowed_url("https://example.com/").unwrap(),
///         DefaultPlugins,
///     ))
/// # ;
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
/// [target.'cfg(not(target_family = "wasm"))'.dependencies]
/// ureq = { version = "3", default-features = false, features = ["gzip", "brotli"] }
/// ```
pub struct WebAssetPlugin {
    pub allowed_urls: &'static [Url],
}

impl WebAssetPlugin {
    /// Creates a new `WebAssetPlugin` with a single allowed URL.
    pub fn allowed_url(allowed_url: &str) -> Result<Self, url::ParseError> {
        let vec = vec![Url::from_str(allowed_url)?];

        Ok(Self {
            allowed_urls: Box::leak(Box::new(vec)).as_slice(),
        })
    }

    /// Creates a new `WebAssetPlugin` with multiple allowed URLs.
    pub fn allowed_urls(allowed_urls: &[&str]) -> Result<Self, url::ParseError> {
        let mut vec = Vec::new();
        for allowed_url in allowed_urls {
            vec.push(Url::from_str(allowed_url)?);
        }

        Ok(Self {
            allowed_urls: Box::leak(Box::new(vec)),
        })
    }
}

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<AssetServer>().is_some() {
            error!("WebAssetPlugin must be added before `AssetPlugin` (typically added as part of `DefaultPlugins`)");
        }

        #[cfg(feature = "http")]
        {
            let allowed_urls = self.allowed_urls;
            app.register_asset_source(
                "http",
                AssetSource::build()
                    .with_reader(move || Box::new(WebAssetReader::Http { allowed_urls }))
                    .with_processed_reader(move || Box::new(WebAssetReader::Http { allowed_urls })),
            );
        }

        #[cfg(feature = "https")]
        {
            let allowed_urls = self.allowed_urls;
            app.register_asset_source(
                "https",
                AssetSource::build()
                    .with_reader(move || Box::new(WebAssetReader::Https { allowed_urls }))
                    .with_processed_reader(move || {
                        Box::new(WebAssetReader::Https { allowed_urls })
                    }),
            );
        }
    }
}

/// Asset reader that treats paths as urls to load assets from.
pub enum WebAssetReader {
    /// Unencrypted connections.
    Http { allowed_urls: &'static [Url] },
    /// Use TLS for setting up connections.
    Https { allowed_urls: &'static [Url] },
}

impl WebAssetReader {
    fn make_uri(&self, path: &Path) -> Result<PathBuf, AssetReaderError> {
        let (prefix, allowed_urls) = match self {
            Self::Http { allowed_urls } => ("http://", allowed_urls),
            Self::Https { allowed_urls } => ("https://", allowed_urls),
        };

        let pathbuf = PathBuf::from(prefix).join(path);
        let Ok(normalized) = normalize_path_lexically(&pathbuf) else {
            return Err(AssetReaderError::NotAllowed(path.to_path_buf()));
        };

        if !allowed_urls
            .iter()
            .any(|url| normalized.starts_with(url.as_str()))
        {
            return Err(AssetReaderError::NotAllowed(path.to_path_buf()));
        }

        Ok(pathbuf)
    }

    /// See [`crate::io::get_meta_path`]
    fn make_meta_uri(&self, path: &Path) -> Result<PathBuf, AssetReaderError> {
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
async fn get(path: Result<PathBuf, AssetReaderError>) -> Result<Box<dyn Reader>, AssetReaderError> {
    let path = path?;

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
        Err(AssetReaderError::NotFound(self.make_uri(path)?))
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

    use std::vec;

    use super::*;

    impl WebAssetReader {
        fn http(allowed_url: &str) -> Self {
            Self::Http {
                allowed_urls: Box::leak(Box::new(vec![Url::from_str(allowed_url).unwrap()]))
                    .as_slice(),
            }
        }
        fn https(allowed_url: &str) -> Self {
            Self::Https {
                allowed_urls: Box::leak(Box::new(vec![Url::from_str(allowed_url).unwrap()]))
                    .as_slice(),
            }
        }
    }

    #[test]
    fn make_http_uri() {
        assert_eq!(
            WebAssetReader::http("http://example.com")
                .make_uri(Path::new("example.com/favicon.png"))
                .unwrap()
                .to_str()
                .unwrap(),
            "http://example.com/favicon.png"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            WebAssetReader::https("https://example.com")
                .make_uri(Path::new("example.com/favicon.png"))
                .unwrap()
                .to_str()
                .unwrap(),
            "https://example.com/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            WebAssetReader::http("http://example.com")
                .make_meta_uri(Path::new("example.com/favicon.png"))
                .unwrap()
                .to_str()
                .unwrap(),
            "http://example.com/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            WebAssetReader::https("https://example.com")
                .make_meta_uri(Path::new("example.com/favicon.png"))
                .unwrap()
                .to_str()
                .unwrap(),
            "https://example.com/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_without_extension_meta_uri() {
        assert_eq!(
            WebAssetReader::https("https://example.com")
                .make_meta_uri(Path::new("example.com/favicon"))
                .unwrap()
                .to_str()
                .unwrap(),
            "https://example.com/favicon.meta"
        );
    }

    #[test]
    fn make_disallowed_uri_fails() {
        let path = Path::new("example.com/favicon.png");

        let error = WebAssetReader::http("http://example.net/")
            .make_uri(path)
            .expect_err("should be an error");

        // This is written weirdly because AssetReaderError can't impl PartialEq
        assert!(matches!(
            error,
            AssetReaderError::NotAllowed(p) if p == path.to_path_buf()
        ));
    }

    // This test mostly checks that the url crate adds a trailing slash to the domain.
    #[test]
    fn enforce_trailing_slash_on_domain() {
        let reader = WebAssetReader::http("http://example.co");

        let path = Path::new("example.com/favicon.png");
        assert!(matches!(
            reader.make_uri(path).expect_err("should be an error"),
            AssetReaderError::NotAllowed(p) if p == path.to_path_buf()
        ));

        let path = Path::new("example.co/favicon.png");
        assert_eq!(
            reader.make_uri(path).unwrap().to_str().unwrap(),
            "http://example.co/favicon.png"
        );
    }

    // This test mostly checks that PathBuf checks path prefixes
    #[test]
    fn enforce_trailing_slash_on_path() {
        let reader = WebAssetReader::http("http://example.com/ima");

        let path = Path::new("example.com/images/favicon.png");
        assert!(matches!(
            reader.make_uri(path).expect_err("should be an error"),
            AssetReaderError::NotAllowed(p) if p == path.to_path_buf()
        ));

        let path = Path::new("example.com/ima/favicon.png");
        assert_eq!(
            reader.make_uri(path).unwrap().to_str().unwrap(),
            "http://example.com/ima/favicon.png"
        );
    }

    #[test]
    fn block_path_traversal() {
        let reader = WebAssetReader::http("http://example.com/images");

        let path = Path::new("example.com/images/../favicon.png");
        assert!(matches!(
            reader.make_uri(path).expect_err("should be an error"),
            AssetReaderError::NotAllowed(p) if p == path.to_path_buf()
        ));

        let path = Path::new("example.com/images/extra/../favicon.png");
        assert_eq!(
            reader.make_uri(path).unwrap().to_str().unwrap(),
            "http://example.com/images/extra/../favicon.png"
        );
    }

    #[test]
    fn fail_https_on_http() {
        let reader = WebAssetReader::http("https://example.com/");

        let path = Path::new("example.com/favicon.png");
        assert!(matches!(
            reader.make_uri(path).expect_err("should be an error"),
            AssetReaderError::NotAllowed(p) if p == path.to_path_buf()
        ));
    }
}
