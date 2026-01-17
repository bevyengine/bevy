use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWriter, AssetWriterError, PathStream,
    Reader, ReaderNotSeekableError, SeekableReader, Writer,
};
use async_fs::{read_dir, File};
use futures_lite::StreamExt;

use alloc::{borrow::ToOwned, boxed::Box};
use std::path::Path;

use super::{FileAssetReader, FileAssetWriter};

impl Reader for File {
    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Ok(self)
    }
}

impl AssetReader for FileAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        File::open(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AssetReaderError::NotFound(full_path.to_path_buf())
            } else {
                e.into()
            }
        })
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        let meta_path = get_meta_path(full_path);
        File::open(&meta_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AssetReaderError::NotFound(meta_path)
            } else {
                e.into()
            }
        })
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        match read_dir(&full_path).await {
            Ok(read_dir) => {
                let root_path = self.root_path.clone();
                let mapped_stream = read_dir.filter_map(move |f| {
                    f.ok().and_then(|dir_entry| {
                        let path = dir_entry.path();
                        // filter out meta files as they are not considered assets
                        if let Some(ext) = path.extension().and_then(|e| e.to_str())
                            && ext.eq_ignore_ascii_case("meta")
                        {
                            return None;
                        }
                        // filter out hidden files. they are not listed by default but are directly targetable
                        if path
                            .file_name()
                            .and_then(|file_name| file_name.to_str())
                            .map(|file_name| file_name.starts_with('.'))
                            .unwrap_or_default()
                        {
                            return None;
                        }
                        let relative_path = path.strip_prefix(&root_path).unwrap();
                        Some(relative_path.to_owned())
                    })
                });
                let read_dir: Box<PathStream> = Box::new(mapped_stream);
                Ok(read_dir)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(AssetReaderError::NotFound(full_path.to_path_buf()))
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        let metadata = full_path
            .metadata()
            .map_err(|_e| AssetReaderError::NotFound(full_path.to_path_buf()))?;
        Ok(metadata.file_type().is_dir())
    }
}

impl AssetWriter for FileAssetWriter {
    async fn write<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        if let Some(parent) = full_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }
        let file = File::create(&full_path).await?;
        let writer: Box<Writer> = Box::new(file);
        Ok(writer)
    }

    async fn write_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        let meta_path = get_meta_path(full_path);
        if let Some(parent) = meta_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }
        let file = File::create(&meta_path).await?;
        let writer: Box<Writer> = Box::new(file);
        Ok(writer)
    }

    async fn remove<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        async_fs::remove_file(full_path).await?;
        Ok(())
    }

    async fn remove_meta<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        let meta_path = get_meta_path(full_path);
        async_fs::remove_file(meta_path).await?;
        Ok(())
    }

    async fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        let (path_buf_old, path_buf_new);
        let full_old_path = if old_path.is_absolute() {
            old_path
        } else {
            path_buf_old = self.root_path.join(old_path);
            &path_buf_old
        };
        let full_new_path = if new_path.is_absolute() {
            new_path
        } else {
            path_buf_new = self.root_path.join(new_path);
            &path_buf_new
        };
        if let Some(parent) = full_new_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }
        async_fs::rename(full_old_path, full_new_path).await?;
        Ok(())
    }

    async fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        let (path_buf_old, path_buf_new);
        let full_old_path = if old_path.is_absolute() {
            old_path
        } else {
            path_buf_old = self.root_path.join(old_path);
            &path_buf_old
        };
        let full_new_path = if new_path.is_absolute() {
            new_path
        } else {
            path_buf_new = self.root_path.join(new_path);
            &path_buf_new
        };
        let old_meta_path = get_meta_path(full_old_path);
        let new_meta_path = get_meta_path(full_new_path);
        if let Some(parent) = new_meta_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }
        async_fs::rename(old_meta_path, new_meta_path).await?;
        Ok(())
    }

    async fn create_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        async_fs::create_dir_all(full_path).await?;
        Ok(())
    }

    async fn remove_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        async_fs::remove_dir_all(full_path).await?;
        Ok(())
    }

    async fn remove_empty_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        async_fs::remove_dir(full_path).await?;
        Ok(())
    }

    async fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        let path_buf;
        let full_path = if path.is_absolute() {
            path
        } else {
            path_buf = self.root_path.join(path);
            &path_buf
        };
        async_fs::remove_dir_all(&full_path).await?;
        async_fs::create_dir_all(&full_path).await?;
        Ok(())
    }
}
