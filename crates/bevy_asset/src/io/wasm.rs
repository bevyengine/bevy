use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AsyncRead, AsyncSeek, EmptyPathStream,
    LocalStackFuture, PathStream, Reader, ReaderRequiredFeatures, SeekFrom, STACK_FUTURE_SIZE,
};
use alloc::{borrow::ToOwned, boxed::Box, format, vec::Vec};
use core::pin::Pin;
use core::task::{Context, Poll};
use js_sys::{Uint8Array, JSON};
use std::path::{Path, PathBuf};
use tracing::error;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

/// Represents the global object in the JavaScript context
#[wasm_bindgen]
extern "C" {
    /// The [Global](https://developer.mozilla.org/en-US/docs/Glossary/Global_object) object.
    type Global;

    /// The [window](https://developer.mozilla.org/en-US/docs/Web/API/Window) global object.
    #[wasm_bindgen(method, getter, js_name = Window)]
    fn window(this: &Global) -> JsValue;

    /// The [WorkerGlobalScope](https://developer.mozilla.org/en-US/docs/Web/API/WorkerGlobalScope) global object.
    #[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
    fn worker(this: &Global) -> JsValue;
}

/// Reader implementation for loading assets via HTTP in Wasm.
pub struct HttpWasmAssetReader {
    root_path: PathBuf,
}

impl HttpWasmAssetReader {
    /// Creates a new `WasmAssetReader`. The path provided will be used to build URLs to query for assets.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root_path: path.as_ref().to_owned(),
        }
    }
}

fn js_value_to_err(context: &str) -> impl FnOnce(JsValue) -> std::io::Error + '_ {
    move |value| {
        let message = match JSON::stringify(&value) {
            Ok(js_str) => format!("Failed to {context}: {js_str}"),
            Err(_) => {
                format!("Failed to {context} and also failed to stringify the JSValue of the error")
            }
        };

        std::io::Error::other(message)
    }
}

impl HttpWasmAssetReader {
    // Also used by [`WebAssetReader`](crate::web::WebAssetReader)
    pub(crate) async fn fetch_bytes(
        &self,
        path: PathBuf,
    ) -> Result<impl Reader + use<>, AssetReaderError> {
        // The JS global scope includes a self-reference via a specializing name, which can be used to determine the type of global context available.
        let global: Global = js_sys::global().unchecked_into();
        let promise = if !global.window().is_undefined() {
            let window: web_sys::Window = global.unchecked_into();
            window.fetch_with_str(path.to_str().unwrap())
        } else if !global.worker().is_undefined() {
            let worker: web_sys::WorkerGlobalScope = global.unchecked_into();
            worker.fetch_with_str(path.to_str().unwrap())
        } else {
            let error = std::io::Error::other("Unsupported JavaScript global context");
            return Err(AssetReaderError::Io(error.into()));
        };
        let resp_value = JsFuture::from(promise)
            .await
            .map_err(js_value_to_err("fetch path"))?;
        let resp = resp_value
            .dyn_into::<Response>()
            .map_err(js_value_to_err("convert fetch to Response"))?;
        match resp.status() {
            200 => {
                let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
                let bytes = Uint8Array::new(&data);
                let reader = Uint8ArrayReader::new(bytes);
                Ok(reader)
            }
            // Some web servers, including itch.io's CDN, return 403 when a requested file isn't present.
            // TODO: remove handling of 403 as not found when it's easier to configure
            // see https://github.com/bevyengine/bevy/pull/19268#pullrequestreview-2882410105
            403 | 404 => Err(AssetReaderError::NotFound(path)),
            status => Err(AssetReaderError::HttpError(status)),
        }
    }
}

impl AssetReader for HttpWasmAssetReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
        _required_features: ReaderRequiredFeatures,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        let path = self.root_path.join(path);
        self.fetch_bytes(path).await
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let meta_path = get_meta_path(&self.root_path.join(path));
        self.fetch_bytes(meta_path).await
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let stream: Box<PathStream> = Box::new(EmptyPathStream);
        error!("Reading directories is not supported with the HttpWasmAssetReader");
        Ok(stream)
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        error!("Reading directories is not supported with the HttpWasmAssetReader");
        Ok(false)
    }
}

/// An [`AsyncRead`] implementation capable of reading a [`Uint8Array`].
pub struct Uint8ArrayReader {
    array: Uint8Array,
    initial_offset: u32,
}

impl Uint8ArrayReader {
    /// Create a new [`Uint8ArrayReader`] for `array`.
    pub fn new(array: Uint8Array) -> Self {
        Self {
            initial_offset: array.byte_offset(),
            array,
        }
    }
}

impl AsyncRead for Uint8ArrayReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<futures_io::Result<usize>> {
        let array_len = self.array.length();
        let n = u32::min(buf.len() as u32, array_len);
        self.array.subarray(0, n).copy_to(&mut buf[..n as usize]); // NOTE: copy_to will panic if the lengths do not exactly match
        self.array = self.array.subarray(n, array_len);
        Poll::Ready(Ok(n as usize))
    }
}

impl AsyncSeek for Uint8ArrayReader {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _cx: &mut Context,
        seek_from: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        let array_len = self.array.length();
        let current_array_buffer_offset = self.array.byte_offset();
        let array_buffer_end = current_array_buffer_offset + array_len;
        let new_array_buffer_offset = match seek_from {
            SeekFrom::Start(from_start) => self
                .initial_offset
                .saturating_add(u32::try_from(from_start).unwrap_or(u32::MAX))
                .min(array_buffer_end),
            SeekFrom::End(from_end) => {
                if from_end.is_negative() {
                    array_buffer_end
                        .saturating_sub(u32::try_from(from_end.abs()).unwrap_or(u32::MAX))
                        .max(self.initial_offset)
                } else {
                    array_buffer_end
                }
            }
            SeekFrom::Current(from_current) => {
                if from_current.is_negative() {
                    current_array_buffer_offset
                        .saturating_sub(u32::try_from(from_current.abs()).unwrap_or(u32::MAX))
                        .max(self.initial_offset)
                } else {
                    current_array_buffer_offset
                        .saturating_add(u32::try_from(from_current).unwrap_or(u32::MAX))
                        .min(array_buffer_end)
                }
            }
        };
        debug_assert!(new_array_buffer_offset >= self.initial_offset);
        debug_assert!(new_array_buffer_offset <= array_buffer_end);
        self.array = Uint8Array::new_with_byte_offset_and_length(
            self.array.buffer().unchecked_ref(),
            new_array_buffer_offset,
            array_buffer_end - new_array_buffer_offset,
        );
        Poll::Ready(Ok((new_array_buffer_offset - self.initial_offset).into()))
    }
}

impl Reader for Uint8ArrayReader {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> LocalStackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        #[expect(unsafe_code)]
        LocalStackFuture::from(async {
            let n = self.array.length();
            let n_usize = n as usize;

            buf.reserve_exact(n_usize);
            let spare_capacity = buf.spare_capacity_mut();
            debug_assert!(spare_capacity.len() >= n_usize);
            // NOTE: `copy_to_uninit` requires the lengths to match exactly,
            // and `reserve_exact` may reserve more capacity than required.
            self.array.copy_to_uninit(&mut spare_capacity[..n_usize]);
            // SAFETY:
            // * the vector has enough spare capacity for `n` additional bytes due to `reserve_exact` above
            // * the bytes have been initialized due to `copy_to_uninit` above.
            unsafe {
                let new_len = buf.len() + n_usize;
                buf.set_len(new_len);
            }
            self.array = self.array.subarray(n, n);

            Ok(n_usize)
        })
    }
}
