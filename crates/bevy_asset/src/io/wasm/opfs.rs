use crate::io::wasm::Global;
use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWriter, AssetWriterError, PathStream,
    Reader, VecReader, Writer,
};
use async_channel::TrySendError;
use bevy_utils::tracing::{error, info};
use futures_io::AsyncWrite;
use futures_lite::{pin, AsyncReadExt, AsyncWriteExt, FutureExt, Stream, StreamExt};
use js_sys::{ArrayBuffer, AsyncIterator, JsString, Uint8Array, JSON};
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions,
    FileSystemGetFileOptions, FileSystemRemoveOptions, FileSystemWritableFileStream,
    StorageManager,
};

#[wasm_bindgen(inline_js = "export function get_keys_for_handle(a) { return a.keys(); }")]
extern "C" {
    /// Workaround to provide [keys](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemDirectoryHandle/keys)
    fn get_keys_for_handle(a: &FileSystemDirectoryHandle) -> AsyncIterator;
}

fn js_value_to_err(
    context: &str,
    kind: std::io::ErrorKind,
) -> impl FnOnce(JsValue) -> std::io::Error + '_ {
    move |value| {
        let message = match JSON::stringify(&value) {
            Ok(js_str) => format!("JS Failure: '{context}': {js_str}"),
            Err(_) => {
                format!("Failed to {context} and also failed to stringify the JSValue of the error")
            }
        };

        std::io::Error::new(kind, message)
    }
}

/// Get the [`StorageManager`] from the global context. Will return [`None`] if the context is not either
/// standard (e.g., with access to `window`), or a worker.
fn get_storage_manager() -> std::io::Result<StorageManager> {
    let global: Global = js_sys::global().unchecked_into();

    if !global.window().is_undefined() {
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
    }
}

/// Extension method to allow for a more ergonomic handling of [promises](`js_sys::Promise`).
trait IntoJsFuture: Into<JsFuture> {
    /// Convert this [thenable](`js_sys::Promise`) into a [`JsFuture`].
    fn into_js_future(self) -> JsFuture {
        self.into()
    }
}

impl<T: Into<JsFuture>> IntoJsFuture for T {}

/// Get the [`FileSystemDirectoryHandle`] for the root Origin Private File System. See
/// [MDN](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/getDirectory) for details.
///
/// Can fail if a `SecurityError` exception is thrown by the JS runtime.
async fn get_storage_root() -> std::io::Result<FileSystemDirectoryHandle> {
    get_storage_manager()?
        .get_directory()
        .into_js_future()
        .await
        .map_err(js_value_to_err(
            "Cannot get StorageManager",
            std::io::ErrorKind::PermissionDenied,
        ))
        .map(|value| value.unchecked_into())
}

/// Open a directory relative to `start` from a given `path`.
/// Will create directories based on the provided `path` if `create` is `true`.
async fn get_directory(
    start: &FileSystemDirectoryHandle,
    path: impl AsRef<Path>,
    create: bool,
) -> std::io::Result<FileSystemDirectoryHandle> {
    let path = path.as_ref();

    let mut options = FileSystemGetDirectoryOptions::new();
    options.create(create);

    let mut current = start.clone();

    for component in path.components() {
        match component {
            Component::Prefix(x) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Cannot parse path '{path:?}': Prefix '{x:?}' is not supported"),
                ));
            }
            Component::RootDir => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Cannot parse path '{path:?}': Cannot use an absolute path"),
                ));
            }
            Component::ParentDir => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Cannot parse path '{path:?}': Relative traversal up the hierarchy is not supported")));
            }
            Component::CurDir => {
                // No-op
                continue;
            }
            Component::Normal(name) => {
                let Some(name) = name.to_str() else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Cannot parse path '{path:?}': Segment '{name:?}' cannot be used as a UTF-8 string")));
                };

                current = current
                    .get_directory_handle_with_options(name, &options)
                    .into_js_future()
                    .await
                    .map_err(js_value_to_err(
                        "Cannot get Directory Handle",
                        std::io::ErrorKind::Other,
                    ))
                    .map(|value| value.unchecked_into())?;
            }
        }
    }

    Ok(current)
}

/// Get child entries of this directory.
async fn get_entries(start: &FileSystemDirectoryHandle) -> impl Stream<Item = PathBuf> + Unpin {
    struct EntriesStream {
        inner: AsyncIterator,
        current: Option<JsFuture>,
    }

    impl Stream for EntriesStream {
        type Item = PathBuf;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut current = match self.current.take() {
                Some(current) => current,
                None => {
                    let Ok(next) = self.inner.next() else {
                        return Poll::Ready(None);
                    };

                    next.into_js_future()
                }
            };

            match current.poll(cx) {
                Poll::Ready(result) => {
                    let result = result
                        .ok()
                        .and_then(|value| value.dyn_ref::<JsString>().cloned())
                        .map(String::from)
                        .map(PathBuf::from);

                    Poll::Ready(result)
                }
                Poll::Pending => {
                    self.current = Some(current);

                    Poll::Pending
                }
            }
        }
    }

    EntriesStream {
        inner: get_keys_for_handle(start),
        current: None,
    }
}

/// Open a file relative to `start` from a given `path`.
/// Will create directories and the final file based on the provided `path` if `create` is `true`.
async fn get_file(
    start: &FileSystemDirectoryHandle,
    path: impl AsRef<Path>,
    create_file: bool,
    create_path: bool,
) -> std::io::Result<FileSystemFileHandle> {
    let path = path.as_ref();

    let mut components = path.components();

    let Some(file_name) = components.next_back() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Provided path is empty",
        ));
    };

    let Component::Normal(file_name) = file_name else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Cannot parse path '{path:?}': final component must be a file name"),
        ));
    };

    let Some(file_name) = file_name.to_str() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Cannot parse path '{path:?}': file name '{file_name:?}' cannot be used as a UTF-8 string"
            ),
        ));
    };

    get_directory(start, components.collect::<PathBuf>(), create_path)
        .await?
        .get_file_handle_with_options(file_name, &{
            let mut options = FileSystemGetFileOptions::new();
            options.create(create_file);
            options
        })
        .into_js_future()
        .await
        .map_err(js_value_to_err(
            format!("File not available: '{path:?}'").as_str(),
            std::io::ErrorKind::NotFound,
        ))
        .map(|value| value.unchecked_into())
}

/// Read the contents of a [file handle](`FileSystemFileHandle`).
async fn read_file<'a>(handle: &FileSystemFileHandle) -> std::io::Result<Box<Reader<'a>>> {
    let file: web_sys::File = handle
        .get_file()
        .into_js_future()
        .await
        .map_err(js_value_to_err(
            "Cannot get File from Handle",
            std::io::ErrorKind::Other,
        ))?
        .unchecked_into();

    let buffer: ArrayBuffer = file
        .array_buffer()
        .into_js_future()
        .await
        .map_err(js_value_to_err(
            "Cannot get Buffer from File",
            std::io::ErrorKind::Other,
        ))?
        .unchecked_into();

    let bytes = Uint8Array::new(&buffer).to_vec();

    Ok(Box::new(VecReader::new(bytes)))
}

async fn write_file(handle: &FileSystemFileHandle) -> std::io::Result<Box<Writer>> {
    enum Command {
        Write(Box<[u8]>, Waker),
        Flush(Waker),
        Close(Waker),
    }

    struct FileStreamWriter {
        commands: async_channel::Sender<Command>,
    }

    impl AsyncWrite for FileStreamWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            match self.commands.try_send(Command::Write(
                buf.to_owned().into_boxed_slice(),
                cx.waker().clone(),
            )) {
                Ok(()) => Poll::Ready(Ok(buf.len())),
                Err(TrySendError::Closed(..)) => Poll::Ready(Ok(0)),
                Err(TrySendError::Full(..)) => Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Could not send write request to writer",
                ))),
            }
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            match self.commands.try_send(Command::Flush(cx.waker().clone())) {
                Ok(()) => Poll::Ready(Ok(())),
                Err(TrySendError::Closed(..)) => Poll::Ready(Ok(())),
                Err(TrySendError::Full(..)) => Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Could not send flush request to writer",
                ))),
            }
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            match self.commands.try_send(Command::Close(cx.waker().clone())) {
                Ok(()) => Poll::Pending,
                Err(TrySendError::Closed(..)) => Poll::Ready(Ok(())),
                Err(TrySendError::Full(..)) => Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Could not send close request to writer",
                ))),
            }
        }
    }

    let stream: FileSystemWritableFileStream = handle
        .create_writable()
        .into_js_future()
        .await
        .map_err(js_value_to_err(
            "Cannot get Create Writable Stream",
            std::io::ErrorKind::Other,
        ))?
        .unchecked_into();

    let (sender, receiver) = async_channel::unbounded::<Command>();

    spawn_local(async move {
        info!("Starting a writer...");
        let stream = stream;
        let receiver = receiver;
        pin!(receiver);

        let maybe_waker = loop {
            let Some(command) = receiver.next().await else {
                break None;
            };

            match command {
                Command::Write(buf, waker) => {
                    info!("Writing {:?}", buf);
                    let Ok(promise) = stream.write_with_u8_array(&buf) else {
                        error!("Cannot Write to Stream!");
                        break None;
                    };

                    let Ok(_) = promise.into_js_future().await else {
                        error!("Cannot Write to Stream!");
                        break None;
                    };

                    waker.wake();
                }
                Command::Flush(waker) => {
                    info!("Flushing");
                    waker.wake();
                }
                Command::Close(waker) => {
                    info!("Closing");
                    let Ok(_) = stream.close().into_js_future().await else {
                        error!("Cannot Close Stream!");
                        break None;
                    };

                    break Some(waker);
                }
            }
        };

        drop(receiver);

        if let Some(waker) = maybe_waker {
            waker.wake();
        } else {
            if stream.close().into_js_future().await.is_err() {
                error!("Stream was closed unexpectedly and could not be closed properly.");
            }
        }
    });

    Ok(Box::new(FileStreamWriter { commands: sender }))
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

/// Bevy compatible wrapper for the [Origin Private File System API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)
pub struct OriginPrivateFileSystem {
    root: PathBuf,
}

impl OriginPrivateFileSystem {
    /// Constructs a new [`OriginPrivateFileSystem`] with the provided shadow-root.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl AssetReader for OriginPrivateFileSystem {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let handle = get_file(&shadow_root, path, false, false)
            .await
            .map_err(|error| AssetReaderError::NotFound(path.to_owned()))?;
        let reader = read_file(&handle).await?;

        Ok(reader)
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let path = &get_meta_path(path);
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let handle = get_file(&shadow_root, path, false, false)
            .await
            .map_err(|error| AssetReaderError::NotFound(path.to_owned()))?;
        let reader = read_file(&handle).await?;

        Ok(reader)
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        struct VecStream<T> {
            inner: Box<[T]>,
            cursor: usize,
        }

        impl<T: Clone> Stream for VecStream<T> {
            type Item = T;

            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                let item = self.inner.get(self.cursor).cloned();

                if item.is_some() {
                    self.cursor += 1;
                }

                Poll::Ready(item)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let remaining = self.inner.len().saturating_sub(self.cursor);

                (remaining, Some(remaining))
            }
        }

        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let handle = get_directory(&shadow_root, path, false).await?;

        let mut entries = get_entries(&handle).await;
        let mut final_entries = Vec::new();

        while let Some(entry) = entries.next().await {
            final_entries.push(entry);
        }

        Ok(Box::new(VecStream {
            inner: final_entries.into_boxed_slice(),
            cursor: 0,
        }))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let result = get_directory(&shadow_root, path, false).await.is_ok();

        Ok(result)
    }
}

impl AssetWriter for OriginPrivateFileSystem {
    async fn write<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let handle = get_file(&shadow_root, path, true, true).await?;
        let writer = write_file(&handle).await?;

        Ok(writer)
    }

    async fn write_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        self.write(&get_meta_path(path)).await
    }

    async fn remove<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, false).await?;
        let _ = get_file(&shadow_root, path, false, false).await?;

        let mut components = path.components();

        let Some(entry) = components.next_back() else {
            return Err(AssetWriterError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Provided path is empty",
            )));
        };

        let Component::Normal(entry) = entry else {
            return Err(AssetWriterError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Cannot parse path '{path:?}': final component must be an entry name"),
            )));
        };

        let Some(entry) = entry.to_str() else {
            return Err(AssetWriterError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Cannot parse path '{path:?}': entry name '{entry:?}' cannot be used as a UTF-8 string"
                ),
            )));
        };

        let parent_handle =
            get_directory(&shadow_root, components.collect::<PathBuf>(), false).await?;

        remove_entry(&parent_handle, entry).await?;

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
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let _ = get_directory(&shadow_root, path, true).await?;

        let mut components = path.components();

        let Some(entry) = components.next_back() else {
            return Err(AssetWriterError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Provided path is empty",
            )));
        };

        let Component::Normal(entry) = entry else {
            return Err(AssetWriterError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Cannot parse path '{path:?}': final component must be an entry name"),
            )));
        };

        let Some(entry) = entry.to_str() else {
            return Err(AssetWriterError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Cannot parse path '{path:?}': entry name '{entry:?}' cannot be used as a UTF-8 string"
                ),
            )));
        };

        let parent_handle =
            get_directory(&shadow_root, components.collect::<PathBuf>(), false).await?;

        remove_entry(&parent_handle, entry).await?;

        Ok(())
    }

    async fn remove_empty_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let handle = get_directory(&shadow_root, path, true).await?;
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
        let root = get_storage_root().await?;
        let shadow_root = get_directory(&root, &self.root, true).await?;
        let handle = get_directory(&shadow_root, path, true).await?;
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
