use crate::io::wasm::Global;
use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWriter, AssetWriterError, PathStream,
    Reader, Writer,
};
use futures_lite::{AsyncReadExt, AsyncWriteExt, Stream, StreamExt};
use js_sys::{JsString, JSON};
use std::path::{Component, Path, PathBuf};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions,
    FileSystemGetFileOptions, FileSystemRemoveOptions, FileSystemWritableFileStream,
};

use utils::*;

/// Bevy compatible wrapper for the [Origin Private File System API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)
pub struct OriginPrivateFileSystem {
    root: Vec<String>,
}

impl OriginPrivateFileSystem {
    /// Constructs a new [`OriginPrivateFileSystem`] with the provided shadow-root.
    pub fn new(root: PathBuf) -> Self {
        let root = Self::canonical(&root)
            .expect("Provided path is not valid")
            .into_iter()
            .map(|component| component.to_owned())
            .collect();

        Self { root }
    }

    /// Constructs a canonical path (as components) from the provided `path`.
    pub(crate) fn canonical<'a>(path: &'a Path) -> std::io::Result<Vec<&'a str>> {
        let mut canonical_path = Vec::new();

        for component in path.components() {
            match component {
                Component::Prefix(x) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Cannot parse path '{path:?}': Prefix '{x:?}' is not supported"),
                    ));
                }
                Component::ParentDir => {
                    let Some(_) = canonical_path.pop() else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!(
                                "Cannot parse path '{path:?}': Cannot get parent directory of root"
                            ),
                        ));
                    };
                }
                Component::RootDir => {
                    let _ = canonical_path.drain(..);
                }
                Component::CurDir => {
                    // No-op
                    continue;
                }
                Component::Normal(name) => {
                    let Some(name) = name.to_str() else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("Cannot parse path '{path:?}': Segment '{name:?}' cannot be used as a UTF-8 string"),
                        ));
                    };

                    canonical_path.push(name);
                }
            }
        }

        Ok(canonical_path)
    }

    /// Get the [`FileSystemDirectoryHandle`] for the root directory pointed to by `self.root`.
    pub(crate) async fn shadow_root(&self) -> std::io::Result<FileSystemDirectoryHandle> {
        let global: Global = js_sys::global().unchecked_into();

        let storage_manager = if !global.window().is_undefined() {
            let window: web_sys::Window = global.unchecked_into();
            Ok(window.navigator().storage())
        } else if !global.worker().is_undefined() {
            let worker: web_sys::WorkerGlobalScope = global.unchecked_into();
            Ok(worker.navigator().storage())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Unsupported global context",
            ))
        }?;

        let root = storage_manager
            .get_directory()
            .into_js_future()
            .await
            .map_err(js_value_to_err(
                "Cannot get StorageManager",
                std::io::ErrorKind::PermissionDenied,
            ))
            .map(|value| value.unchecked_into())?;

        get_directory(&root, self.root.iter().map(|value| value.as_str()), true).await
    }
}

impl AssetReader for OriginPrivateFileSystem {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let shadow_root = self.shadow_root().await?;

        let reader = get_file(&shadow_root, Self::canonical(path)?, false, false)
            .await
            .map_err(|_error| AssetReaderError::NotFound(path.to_owned()))?
            .get_file()
            .into_js_future()
            .await
            .map_err(js_value_to_err(
                "Cannot get File from Handle",
                std::io::ErrorKind::Other,
            ))?
            .unchecked_into::<web_sys::File>()
            .get_async_reader()
            .await?;

        Ok(Box::new(reader))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let path = &get_meta_path(path);
        let shadow_root = self.shadow_root().await?;

        let reader = get_file(&shadow_root, Self::canonical(path)?, false, false)
            .await
            .map_err(|_error| AssetReaderError::NotFound(path.to_owned()))?
            .get_file()
            .into_js_future()
            .await
            .map_err(js_value_to_err(
                "Cannot get File from Handle",
                std::io::ErrorKind::Other,
            ))?
            .unchecked_into::<web_sys::File>()
            .get_async_reader()
            .await?;

        Ok(Box::new(reader))
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let shadow_root = self.shadow_root().await?;
        let handle = get_directory(&shadow_root, Self::canonical(path)?, false).await?;
        let entries = get_entries(&handle).await;

        let (stream, task) = IndirectStream::wrap(entries);

        spawn_local(task);

        Ok(Box::new(stream))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let shadow_root = self.shadow_root().await?;
        let result = get_directory(&shadow_root, Self::canonical(path)?, false)
            .await
            .is_ok();

        Ok(result)
    }
}

impl AssetWriter for OriginPrivateFileSystem {
    async fn write<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        let shadow_root = self.shadow_root().await?;
        let handle = get_file(&shadow_root, Self::canonical(path)?, true, true).await?;

        let stream: FileSystemWritableFileStream = handle
            .create_writable()
            .into_js_future()
            .await
            .map_err(js_value_to_err(
                "Cannot get Create Writable Stream",
                std::io::ErrorKind::Other,
            ))?
            .unchecked_into();

        Ok(Box::new(stream.into_async_writer()))
    }

    async fn write_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        self.write(&get_meta_path(path)).await
    }

    async fn remove<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let shadow_root = self.shadow_root().await?;
        let canon = Self::canonical(path)?;
        let _ = get_file(&shadow_root, canon.iter().copied(), false, false).await?;

        let [parent @ .., file] = canon.as_slice() else {
            unreachable!("path valid based on above guard");
        };

        let parent_handle = get_directory(&shadow_root, parent.iter().copied(), false).await?;

        remove_entry(&parent_handle, file).await?;

        Ok(())
    }

    async fn remove_meta<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        self.remove(&get_meta_path(path)).await
    }

    async fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        let mut buffer = Vec::new();

        self.read(old_path)
            .await
            .map_err(|error| {
                AssetWriterError::from(std::io::Error::new(std::io::ErrorKind::Other, error))
            })?
            .read_to_end(&mut buffer)
            .await?;
        self.write(new_path).await?.write(&buffer).await?;
        self.remove(old_path).await?;

        Ok(())
    }

    async fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        self.rename(&get_meta_path(old_path), &get_meta_path(new_path))
            .await
    }

    async fn remove_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let shadow_root = self.shadow_root().await?;
        let canon = Self::canonical(path)?;
        let _ = get_directory(&shadow_root, canon.iter().copied(), true).await?;

        let [parent @ .., directory] = canon.as_slice() else {
            unreachable!("path valid based on above guard");
        };

        let parent_handle = get_directory(&shadow_root, parent.iter().copied(), false).await?;

        remove_entry(&parent_handle, directory).await?;

        Ok(())
    }

    async fn remove_empty_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let shadow_root = self.shadow_root().await?;
        let canon = Self::canonical(path)?;
        let handle = get_directory(&shadow_root, canon, true).await?;
        let mut stream = get_entries(&handle).await;

        if stream.next().await.is_some() {
            return Err(AssetWriterError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Directory is not empty",
            )));
        }

        self.remove_directory(path).await
    }

    async fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        let shadow_root = self.shadow_root().await?;
        let handle = get_directory(&shadow_root, Self::canonical(path)?, true).await?;
        let mut stream = get_entries(&handle).await;

        while let Some(entry) = stream.next().await {
            let Some(entry) = entry.to_str() else {
                unreachable!("Only valid UTF-8 is storable in the Origin Private File System")
            };

            remove_entry(&handle, entry).await?;
        }

        Ok(())
    }
}

/// Reduced boilerplate for generating [error](std::io::Error) values.
fn js_value_to_err(
    context: &str,
    kind: std::io::ErrorKind,
) -> impl FnOnce(JsValue) -> std::io::Error + '_ {
    move |value| {
        let error = JSON::stringify(&value)
            .map(String::from)
            .ok()
            .unwrap_or_else(|| "failed to stringify the JSValue of the error".to_owned());

        let message = format!("JS Failure: '{context}': {error}");

        std::io::Error::new(kind, message)
    }
}

/// Open a directory relative to `start` from a given `path`.
/// Will create directories based on the provided `path` if `create` is `true`.
async fn get_directory(
    start: &FileSystemDirectoryHandle,
    path: impl IntoIterator<Item = &str>,
    create: bool,
) -> std::io::Result<FileSystemDirectoryHandle> {
    let options = {
        let mut options = FileSystemGetDirectoryOptions::new();
        options.create(create);
        options
    };

    let mut current = start.clone();

    for component in path {
        current = current
            .get_directory_handle_with_options(component, &options)
            .into_js_future()
            .await
            .map_err(js_value_to_err(
                "Cannot get Directory Handle",
                std::io::ErrorKind::NotFound,
            ))
            .map(|value| value.unchecked_into())?;
    }

    Ok(current)
}

/// Get child entries of this directory.
async fn get_entries(start: &FileSystemDirectoryHandle) -> impl Stream<Item = PathBuf> + Unpin {
    JsStream::from(start.keys())
        .flat_map(|result| futures_lite::stream::iter(result.ok()))
        .flat_map(|value| futures_lite::stream::iter(value.dyn_into::<JsString>().ok()))
        .map(String::from)
        .map(PathBuf::from)
}

/// Open a file relative to `start` from a given `path`.
/// Will create directories and the final file based on the provided `path` if `create` is `true`.
async fn get_file(
    start: &FileSystemDirectoryHandle,
    path: impl IntoIterator<Item = &str>,
    create_file: bool,
    create_path: bool,
) -> std::io::Result<FileSystemFileHandle> {
    let mut current = start.clone();

    let mut iter = path.into_iter().peekable();

    let options = {
        let mut options = FileSystemGetDirectoryOptions::new();
        options.create(create_path);
        options
    };

    let file_name = loop {
        let Some(path) = iter.next() else {
            break None;
        };

        if iter.peek().is_none() {
            break Some(path);
        };

        current = current
            .get_directory_handle_with_options(path, &options)
            .into_js_future()
            .await
            .map_err(js_value_to_err(
                "Cannot get Directory Handle",
                std::io::ErrorKind::NotFound,
            ))
            .map(|value| value.unchecked_into())?;
    };

    let Some(file_name) = file_name else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Provided path is empty",
        ));
    };

    current
        .get_file_handle_with_options(file_name, &{
            let mut options = FileSystemGetFileOptions::new();
            options.create(create_file);
            options
        })
        .into_js_future()
        .await
        .map_err(js_value_to_err(
            "File not available",
            std::io::ErrorKind::NotFound,
        ))
        .map(|value| value.unchecked_into())
}

async fn remove_entry(handle: &FileSystemDirectoryHandle, entry: &str) -> std::io::Result<()> {
    handle
        .remove_entry_with_options(entry, &{
            let mut options = FileSystemRemoveOptions::new();
            options.recursive(true);
            options
        })
        .into_js_future()
        .await
        .map_err(js_value_to_err(
            "Cannot remove Directory",
            std::io::ErrorKind::Other,
        ))?
        .is_undefined()
        .then_some(())
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to remove entry",
        ))
}

mod utils {
    use crate::io::wasm::opfs::js_value_to_err;
    use crate::io::VecReader;
    use async_channel::{TryRecvError, TrySendError};
    use bevy_utils::tracing::error;
    use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
    use futures_lite::{pin, FutureExt, Stream, StreamExt};
    use js_sys::{ArrayBuffer, AsyncIterator, IteratorNext, Uint8Array};
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::{spawn_local, JsFuture};
    use web_sys::{Blob, FileSystemDirectoryHandle, FileSystemWritableFileStream};

    /// Extension method to allow for a more ergonomic handling of [promises](`js_sys::Promise`).
    pub(crate) trait IntoJsFuture: Into<JsFuture> {
        /// Convert this [thenable](`js_sys::Promise`) into a [`JsFuture`].
        fn into_js_future(self) -> JsFuture {
            self.into()
        }
    }

    impl<T: Into<JsFuture>> IntoJsFuture for T {}

    /// A [`Stream`] that yields values from an underlying [`AsyncIterator`]
    ///
    /// Based on [`wasm_bindgen_futures::stream::JsStream`](https://github.com/olanod/wasm-bindgen/blob/a8edfb117c79654773cf3d9b4da3e4a01b9884ab/crates/futures/src/stream.rs).
    /// Can be removed once [#2399](https://github.com/rustwasm/wasm-bindgen/issues/2399) is resolved.
    pub(crate) struct JsStream {
        iter: AsyncIterator,
        next: Option<JsFuture>,
        done: bool,
    }

    impl Stream for JsStream {
        type Item = Result<JsValue, JsValue>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.done {
                return Poll::Ready(None);
            }

            let future = match self.next.as_mut() {
                Some(val) => val,
                None => match self.iter.next().map(JsFuture::from) {
                    Ok(val) => {
                        self.next = Some(val);
                        self.next.as_mut().unwrap()
                    }
                    Err(e) => {
                        self.done = true;
                        return Poll::Ready(Some(Err(e)));
                    }
                },
            };

            match Pin::new(future).poll(cx) {
                Poll::Ready(res) => match res {
                    Ok(iter_next) => {
                        let next = iter_next.unchecked_into::<IteratorNext>();
                        if next.done() {
                            self.done = true;
                            Poll::Ready(None)
                        } else {
                            self.next.take();
                            Poll::Ready(Some(Ok(next.value())))
                        }
                    }
                    Err(e) => {
                        self.done = true;
                        Poll::Ready(Some(Err(e)))
                    }
                },
                Poll::Pending => Poll::Pending,
            }
        }
    }

    impl From<AsyncIterator> for JsStream {
        fn from(value: AsyncIterator) -> Self {
            Self {
                iter: value,
                next: None,
                done: false,
            }
        }
    }

    /// Extension trait providing access to the async iterator methods on [`FileSystemDirectoryHandle`]
    /// which are currently missing from [`wasm-bindgen`](`wasm_bindgen`)
    pub(crate) trait FileSystemDirectoryHandleExt {
        fn keys(&self) -> AsyncIterator;
    }

    impl FileSystemDirectoryHandleExt for FileSystemDirectoryHandle {
        /// The `keys()` method.
        ///
        /// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemDirectoryHandle/keys)
        ///
        /// *This API requires the following crate features to be activated: `FileSystemDirectoryHandle`*
        fn keys(&self) -> AsyncIterator {
            #[wasm_bindgen(
                inline_js = "export function get_keys_for_handle(a) { return a.keys(); }"
            )]
            extern "C" {
                fn get_keys_for_handle(a: &FileSystemDirectoryHandle) -> AsyncIterator;
            }

            get_keys_for_handle(self)
        }
    }

    /// Uses channels to create a [`Send`] + [`Sync`] wrapper around a [`Stream`].
    pub(crate) struct IndirectStream<T> {
        request: Pin<Box<async_channel::Sender<Waker>>>,
        response: Pin<Box<async_channel::Receiver<T>>>,
    }

    impl<T> Stream for IndirectStream<T> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match self.response.try_recv() {
                Ok(value) => Poll::Ready(Some(value)),
                Err(TryRecvError::Closed) => Poll::Ready(None),
                Err(TryRecvError::Empty) => match self.request.try_send(cx.waker().clone()) {
                    Ok(_) | Err(TrySendError::Full(_)) => Poll::Pending,
                    Err(TrySendError::Closed(_)) => Poll::Ready(None),
                },
            }
        }
    }

    impl<T: Send + Sync + 'static> IndirectStream<T> {
        /// Take the provided `stream` and split it into a [`Send`] + [`Sync`] stream and a backing task.
        /// It is the callers responsibility to ensure the task is run on an appropriate runtime.
        ///
        /// Internally uses [async channels](`async_channel`) to request values from the stream whilst
        /// also passing an appropriate [`Waker`].
        pub(crate) fn wrap(
            stream: impl Stream<Item = T> + 'static,
        ) -> (Self, impl std::future::Future<Output = ()>) {
            let (send_waker, receive_waker) = async_channel::bounded::<Waker>(1);
            let (send_value, receive_value) = async_channel::bounded::<T>(1);

            let task = async move {
                pin!(stream);
                pin!(receive_waker);
                pin!(send_value);

                while let Some(waker) = receive_waker.next().await {
                    if let Some(item) = stream.next().await {
                        if let Ok(_) = send_value.send(item).await {
                            waker.wake();
                            continue;
                        }
                    }

                    waker.wake();
                    break;
                }
            };

            let stream = Self {
                request: Box::into_pin(Box::new(send_waker)),
                response: Box::into_pin(Box::new(receive_value)),
            };

            (stream, task)
        }
    }

    pub(crate) trait BlobExt {
        async fn get_async_reader(
            &self,
        ) -> std::io::Result<impl AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static>;
    }

    impl BlobExt for Blob {
        async fn get_async_reader(
            &self,
        ) -> std::io::Result<impl AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static> {
            let buffer: ArrayBuffer = self
                .array_buffer()
                .into_js_future()
                .await
                .map_err(js_value_to_err(
                    "Cannot get Buffer from Blob",
                    std::io::ErrorKind::Other,
                ))?
                .unchecked_into();

            let bytes = Uint8Array::new(&buffer).to_vec();

            Ok(VecReader::new(bytes))
        }
    }

    pub(crate) trait FileSystemWritableFileStreamExt {
        fn into_async_writer(self) -> impl AsyncWrite + Unpin + Send + Sync;
    }

    impl FileSystemWritableFileStreamExt for FileSystemWritableFileStream {
        /// Create an [async writer](`AsyncWrite`) from this [`FileSystemWritableFileStream`].
        fn into_async_writer(self) -> impl AsyncWrite + Unpin + Send + Sync {
            struct FileStreamWriter {
                writes: async_channel::Sender<Box<[u8]>>,
                wake_on_closed: async_channel::Sender<Waker>,
            }

            impl AsyncWrite for FileStreamWriter {
                fn poll_write(
                    self: Pin<&mut Self>,
                    _cx: &mut Context<'_>,
                    buf: &[u8],
                ) -> Poll<std::io::Result<usize>> {
                    if self.writes.is_full() {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::BrokenPipe,
                            "Could not send write request to writer",
                        )));
                    }

                    if self.writes.is_closed() {
                        return Poll::Ready(Ok(0));
                    }

                    let write = buf.to_owned().into_boxed_slice();

                    let Ok(_) = self.writes.try_send(write) else {
                        return Poll::Ready(Ok(0));
                    };

                    Poll::Ready(Ok(buf.len()))
                }

                fn poll_flush(
                    self: Pin<&mut Self>,
                    _cx: &mut Context<'_>,
                ) -> Poll<std::io::Result<()>> {
                    return Poll::Ready(Ok(()));
                }

                fn poll_close(
                    self: Pin<&mut Self>,
                    cx: &mut Context<'_>,
                ) -> Poll<std::io::Result<()>> {
                    if self.wake_on_closed.is_closed() {
                        return Poll::Ready(Ok(()));
                    }

                    match self.wake_on_closed.try_send(cx.waker().clone()) {
                        Ok(_) => Poll::Pending,
                        Err(TrySendError::Closed(_)) => Poll::Ready(Ok(())),
                        Err(TrySendError::Full(_)) => Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::BrokenPipe,
                            "Could not send close request to AsyncWrite stream",
                        ))),
                    }
                }
            }

            let (send_bytes, receive_bytes) = async_channel::unbounded::<Box<[u8]>>();
            let (send_waker, receive_waker) = async_channel::unbounded::<Waker>();

            spawn_local(async move {
                pin!(receive_bytes);
                pin!(receive_waker);

                while let Some(buf) = receive_bytes.next().await {
                    if let Ok(promise) = self.write_with_u8_array(&buf) {
                        if let Ok(_) = promise.into_js_future().await {
                            continue;
                        }
                    }

                    break;
                }

                receive_bytes.close();

                if self.close().into_js_future().await.is_err() {
                    error!("FileSystemWritableFileStream could not be closed properly.");
                }

                while let Ok(waker) = receive_waker.try_recv() {
                    waker.wake()
                }
            });

            FileStreamWriter {
                writes: send_bytes,
                wake_on_closed: send_waker,
            }
        }
    }
}
