use crate::io::{AssetReader, AssetReaderError, Reader};
use crate::io::{AssetSource, PathStream};
use crate::AssetApp;
use alloc::boxed::Box;
use bevy_app::App;
use bevy_tasks::ConditionalSendFuture;
use std::path::{Path, PathBuf};

/// Adds the `http` and `https` asset sources to the app.
/// Any asset path that begins with `http` or `https` will be loaded from the web
/// via `fetch`(wasm) or `ureq`(native).
pub fn http_source_plugin(app: &mut App) {
    app.register_asset_source(
        "http",
        AssetSource::build().with_reader(|| Box::new(HttpSourceAssetReader::Http)),
    );
    app.register_asset_source(
        "https",
        AssetSource::build().with_reader(|| Box::new(HttpSourceAssetReader::Https)),
    );
}

/// Asset reader that treats paths as urls to load assets from.
pub enum HttpSourceAssetReader {
    /// Unencrypted connections.
    Http,
    /// Use TLS for setting up connections.
    Https,
}

impl HttpSourceAssetReader {
    fn make_uri(&self, path: &Path) -> PathBuf {
        PathBuf::from(match self {
            Self::Http => "http://",
            Self::Https => "https://",
        })
        .join(path)
    }

    /// See [`crate::io::get_meta_path`]
    fn make_meta_uri(&self, path: &Path) -> Option<PathBuf> {
        let mut uri = self.make_uri(path);
        let mut extension = path.extension()?.to_os_string();
        extension.push(".meta");
        uri.set_extension(extension);
        Some(uri)
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

    #[cfg(feature = "http_source_cache")]
    if let Some(data) = http_asset_cache::try_load_from_cache(str_path)? {
        return Ok(Box::new(VecReader::new(data)));
    }
    use ureq::Agent;

    static AGENT: LazyLock<Agent> = LazyLock::new(|| Agent::config_builder().build().new_agent());

    match AGENT.get(str_path).call() {
        Ok(mut response) => {
            // let mut reader = response.into_reader();
            let mut reader = BufReader::new(response.body_mut().with_config().reader());

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;

            #[cfg(feature = "http_source_cache")]
            http_asset_cache::save_to_cache(str_path, &buffer)?;

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

impl AssetReader for HttpSourceAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<dyn Reader>, AssetReaderError>> {
        get(self.make_uri(path))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader>, AssetReaderError> {
        match self.make_meta_uri(path) {
            Some(uri) => get(uri).await,
            None => Err(AssetReaderError::NotFound(
                "source path has no extension".into(),
            )),
        }
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

/// A naive implementation of an HTTP asset cache that never invalidates.
/// `ureq` currently does not support caching, so this is a simple workaround.
/// It should eventually be replaced by `http-cache` or similar, see [tracking issue](https://github.com/06chaynes/http-cache/issues/91)
#[cfg(feature = "http_source_cache")]
mod http_asset_cache {
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    use std::fs::{self, File};
    use std::io::{self, Read, Write};
    use std::path::PathBuf;

    const CACHE_DIR: &str = ".http-asset-cache";

    fn url_to_hash(url: &str) -> String {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        std::format!("{:x}", hasher.finish())
    }

    pub fn try_load_from_cache(url: &str) -> Result<Option<Vec<u8>>, io::Error> {
        let filename = url_to_hash(url);
        let cache_path = PathBuf::from(CACHE_DIR).join(&filename);

        if cache_path.exists() {
            let mut file = File::open(&cache_path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            Ok(Some(buffer))
        } else {
            Ok(None)
        }
    }

    pub fn save_to_cache(url: &str, data: &[u8]) -> Result<(), io::Error> {
        let filename = url_to_hash(url);
        let cache_path = PathBuf::from(CACHE_DIR).join(&filename);

        fs::create_dir_all(CACHE_DIR).ok();

        let mut cache_file = File::create(&cache_path)?;
        cache_file.write_all(data)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_http_uri() {
        assert_eq!(
            HttpSourceAssetReader::Http
                .make_uri(Path::new("example.com/favicon.png"))
                .to_str()
                .unwrap(),
            "http://example.com/favicon.png"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            HttpSourceAssetReader::Https
                .make_uri(Path::new("example.com/favicon.png"))
                .to_str()
                .unwrap(),
            "https://example.com/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            HttpSourceAssetReader::Http
                .make_meta_uri(Path::new("example.com/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "http://example.com/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            HttpSourceAssetReader::Https
                .make_meta_uri(Path::new("example.com/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "https://example.com/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_without_extension_meta_uri() {
        assert_eq!(
            HttpSourceAssetReader::Https.make_meta_uri(Path::new("example.com/favicon")),
            None
        );
    }
}
