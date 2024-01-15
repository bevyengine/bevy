use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWriter, AssetWriterError, PathStream,
    Reader, Writer,
};
use async_fs::{read_dir, File};
use futures_lite::StreamExt;

use std::path::Path;

use super::{FileAssetReader, FileAssetWriter};

impl AssetReader for FileAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let full_path = self.root_path.join(path);
        match File::open(&full_path).await {
            Ok(file) => {
                let reader: Box<Reader> = Box::new(file);
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
        match File::open(&full_path).await {
            Ok(file) => {
                let reader: Box<Reader> = Box::new(file);
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
        match read_dir(&full_path).await {
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
                let read_dir: Box<PathStream> = Box::new(mapped_stream);
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

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
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
            async_fs::create_dir_all(parent).await?;
        }
        let file = File::create(&full_path).await?;
        let writer: Box<Writer> = Box::new(file);
        Ok(writer)
    }

    async fn write_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Writer>, AssetWriterError> {
        let meta_path = get_meta_path(path);
        let full_path = self.root_path.join(meta_path);
        if let Some(parent) = full_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }
        let file = File::create(&full_path).await?;
        let writer: Box<Writer> = Box::new(file);
        Ok(writer)
    }

    async fn remove<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        async_fs::remove_file(full_path).await?;
        Ok(())
    }

    async fn remove_meta<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let meta_path = get_meta_path(path);
        let full_path = self.root_path.join(meta_path);
        async_fs::remove_file(full_path).await?;
        Ok(())
    }

    async fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        let full_old_path = self.root_path.join(old_path);
        let full_new_path = self.root_path.join(new_path);
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
        let old_meta_path = get_meta_path(old_path);
        let new_meta_path = get_meta_path(new_path);
        let full_old_path = self.root_path.join(old_meta_path);
        let full_new_path = self.root_path.join(new_meta_path);
        if let Some(parent) = full_new_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }
        async_fs::rename(full_old_path, full_new_path).await?;
        Ok(())
    }

    async fn remove_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        async_fs::remove_dir_all(full_path).await?;
        Ok(())
    }

    async fn remove_empty_directory<'a>(&'a self, path: &'a Path) -> Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        async_fs::remove_dir(full_path).await?;
        Ok(())
    }

    async fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<(), AssetWriterError> {
        let full_path = self.root_path.join(path);
        async_fs::remove_dir_all(&full_path).await?;
        async_fs::create_dir_all(&full_path).await?;
        Ok(())
    }
}
