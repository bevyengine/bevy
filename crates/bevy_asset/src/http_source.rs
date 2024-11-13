use crate::io::{AssetReader, AssetReaderError, Reader};
use crate::io::{AssetSource, PathStream};
use crate::AssetApp;
use bevy_app::App;
use bevy_utils::ConditionalSendFuture;
use std::path::{Path, PathBuf};

/// Adds the `http` and `https` asset sources to the app.
/// Any asset path that begins with `http` or `https` will be loaded from the web
/// via `fetch`(wasm) or `surf`(native).
pub fn http_source_plugin(app: &mut App) {
    app.register_asset_source(
        "http",
        AssetSource::build().with_reader(|| Box::new(WebAssetReader::Http)),
    );
    app.register_asset_source(
        "https",
        AssetSource::build().with_reader(|| Box::new(WebAssetReader::Https)),
    );
}

/// Treats paths as urls to load assets from.
pub enum WebAssetReader {
    /// Unencrypted connections.
    Http,
    /// Use TLS for setting up connections.
    Https,
}

impl WebAssetReader {
    fn make_uri(&self, path: &Path) -> PathBuf {
        PathBuf::from(match self {
            Self::Http => "http://",
            Self::Https => "https://",
        })
        .join(path)
    }

    /// See [crate::io::get_meta_path]
    fn make_meta_uri(&self, path: &Path) -> Option<PathBuf> {
        let mut uri = self.make_uri(path);
        let mut extension = path.extension()?.to_os_string();
        extension.push(".meta");
        uri.set_extension(extension);
        Some(uri)
    }
}

#[cfg(target_arch = "wasm32")]
async fn get<'a>(path: PathBuf) -> Result<Box<Reader<'a>>, AssetReaderError> {
    use bevy::asset::io::VecReader;
    use js_sys::Uint8Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    fn js_value_to_err<'a>(
        context: &'a str,
    ) -> impl FnOnce(wasm_bindgen::JsValue) -> std::io::Error + 'a {
        move |value| {
            let message = match js_sys::JSON::stringify(&value) {
                Ok(js_str) => format!("Failed to {context}: {js_str}"),
                Err(_) => {
                    format!(
                        "Failed to {context} and also failed to stringify the JSValue of the error"
                    )
                }
            };

            std::io::Error::new(std::io::ErrorKind::Other, message)
        }
    }

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_str(path.to_str().unwrap()))
        .await
        .map_err(js_value_to_err("fetch path"))?;
    let resp = resp_value
        .dyn_into::<Response>()
        .map_err(js_value_to_err("convert fetch to Response"))?;
    match resp.status() {
        200 => {
            let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
            let bytes = Uint8Array::new(&data).to_vec();
            let reader: Box<Reader> = Box::new(VecReader::new(bytes));
            Ok(reader)
        }
        404 => Err(AssetReaderError::NotFound(path)),
        status => Err(AssetReaderError::Io(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Encountered unexpected HTTP status {status}"),
            )
            .into(),
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn get<'a>(path: PathBuf) -> Result<Box<dyn Reader>, AssetReaderError> {
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use std::io;

    use crate::io::VecReader;
    use http_cache_surf::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
    use surf::StatusCode;

    #[pin_project::pin_project]
    struct ContinuousPoll<T>(#[pin] T);

    impl<T: Future> Future for ContinuousPoll<T> {
        type Output = T::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            // Always wake - blocks on single threaded executor.
            cx.waker().wake_by_ref();

            self.project().0.poll(cx)
        }
    }

    let str_path = path.to_str().ok_or_else(|| {
        AssetReaderError::Io(
            io::Error::new(
                io::ErrorKind::Other,
                format!("non-utf8 path: {}", path.display()),
            )
            .into(),
        )
    })?;

    let req = surf::get(str_path);
    let middleware_client = surf::client().with(Cache(HttpCache {
        mode: CacheMode::Default,
        manager: CACacheManager::default(),
        options: HttpCacheOptions::default(),
    }));

    let mut response = ContinuousPoll(middleware_client.send(req))
        .await
        .map_err(|err| {
            AssetReaderError::Io(
                io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "unexpected status code {} while loading {}: {}",
                        err.status(),
                        path.display(),
                        err.into_inner(),
                    ),
                )
                .into(),
            )
        })?;

    match response.status() {
        StatusCode::Ok => Ok(Box::new(VecReader::new(
            ContinuousPoll(response.body_bytes())
                .await
                .map_err(|_| AssetReaderError::NotFound(path.to_path_buf()))?,
        )) as _),
        StatusCode::NotFound => Err(AssetReaderError::NotFound(path)),
        code => Err(AssetReaderError::Io(
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "unexpected status code {} while loading {}",
                    code,
                    path.display()
                ),
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_http_uri() {
        assert_eq!(
            WebAssetReader::Http
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            WebAssetReader::Https
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            WebAssetReader::Http
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            WebAssetReader::Https
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_without_extension_meta_uri() {
        assert_eq!(
            WebAssetReader::Https.make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon")),
            None
        );
    }
}
