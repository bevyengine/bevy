use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
use futures_lite::Stream;

use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWriter, AssetWriterError, PathStream,
    Reader, Writer,
};

use std::{
    fs::{read_dir, File},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    pin::Pin,
    task::Poll,
};

use super::{FileAssetReader, FileAssetWriter};

struct FileReader(File);

impl AsyncRead for FileReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let read = this.0.read(buf);
        Poll::Ready(read)
    }
}

impl AsyncSeek for FileReader {
    fn poll_seek(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        pos: std::io::SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        let this = self.get_mut();
        let seek = this.0.seek(pos);
        Poll::Ready(seek)
    }
}

struct FileWriter(File);

impl AsyncWrite for FileWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let wrote = this.0.write(buf);
        Poll::Ready(wrote)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let flushed = this.0.flush();
        Poll::Ready(flushed)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

struct DirReader(Vec<PathBuf>);

impl Stream for DirReader {
    type Item = PathBuf;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Poll::Ready(this.0.pop())
    }
}

impl AssetReader for FileAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let full_path = self.root_path.join(path);
        match File::open(&full_path) {
            Ok(file) => {
                let reader: Box<Reader> = Box::new(FileReader(file));
                Ok(reader)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(AssetReaderError::NotFound(full_path))
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let meta_path = get_meta_path(path);
        let full_path = self.root_path.join(meta_path);
        match File::open(&full_path) {
            Ok(file) => {
                let reader: Box<Reader> = Box::new(FileReader(file));
                Ok(reader)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(AssetReaderError::NotFound(full_path))
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let full_path = self.root_path.join(path);
        match read_dir(&full_path) {
            Ok(read_dir) => {
                let root_path = self.root_path.clone();
                let mapped_stream = read_dir.filter_map(move |f| {
                    f.ok().and_then(|dir_entry| {
                        let path = dir_entry.path();
                        // filter out meta files as they are not considered assets
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext.eq_ignore_ascii_case("meta") {
                                return None;
                            }
                        }
                        let relative_path = path.strip_prefix(&root_path).unwrap();
                        Some(relative_path.to_owned())
                    })
                });
                let read_dir: Box<PathStream> = Box::new(DirReader(mapped_stream.collect()));
                Ok(read_dir)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(AssetReaderError::NotFound(full_path))
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> std::result::Result<bool, AssetReaderError> {
        let full_path = self.root_path.join(path);
        let metadata = full_path
            .metadata()
            .map_err(|_e| AssetReaderError::NotFound(path.to_owned()))?;
        Ok(metadata.file_type().is_dir())
    }
}

impl AssetWriter for FileAssetWriter {
    async fn write<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        let full_path = self.root_path.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(&full_path)?;
        let writer: Box<Writer> = Box::new(FileWriter(file));
        Ok(writer)
    }

    async fn write_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        let meta_path = get_meta_path(path);
        let full_path = self.root_path.join(meta_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(&full_path)?;
        let writer: Box<Writer> = Box::new(FileWriter(file));
        Ok(writer)
    }

    async fn remove<'a>(&'a self, path: &'a Path) -> std::result::Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        std::fs::remove_file(full_path)?;
        Ok(())
    }

    async fn remove_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> std::result::Result<(), AssetWriterError> {
        let meta_path = get_meta_path(path);
        let full_path = self.root_path.join(meta_path);
        std::fs::remove_file(full_path)?;
        Ok(())
    }

    async fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> std::result::Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        std::fs::remove_dir_all(full_path)?;
        Ok(())
    }

    async fn remove_empty_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> std::result::Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        std::fs::remove_dir(full_path)?;
        Ok(())
    }

    async fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> std::result::Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        std::fs::remove_dir_all(&full_path)?;
        std::fs::create_dir_all(&full_path)?;
        Ok(())
    }

    async fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> std::result::Result<(), AssetWriterError> {
        let full_old_path = self.root_path.join(old_path);
        let full_new_path = self.root_path.join(new_path);
        if let Some(parent) = full_new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(full_old_path, full_new_path)?;
        Ok(())
    }

    async fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> std::result::Result<(), AssetWriterError> {
        let old_meta_path = get_meta_path(old_path);
        let new_meta_path = get_meta_path(new_path);
        let full_old_path = self.root_path.join(old_meta_path);
        let full_new_path = self.root_path.join(new_meta_path);
        if let Some(parent) = full_new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(full_old_path, full_new_path)?;
        Ok(())
    }
}
