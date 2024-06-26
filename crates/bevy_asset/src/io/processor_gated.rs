use crate::{
    io::{AssetReader, AssetReaderError, AssetSourceId, PathStream, Reader},
    processor::{AssetProcessorData, ProcessStatus},
    AssetPath,
};
use async_lock::RwLockReadGuardArc;
use bevy_utils::tracing::trace;
use futures_io::{AsyncRead, AsyncSeek};
use std::io::SeekFrom;
use std::task::Poll;
use std::{path::Path, pin::Pin, sync::Arc};

use super::ErasedAssetReader;

/// An [`AssetReader`] that will prevent asset (and asset metadata) read futures from returning for a
/// given path until that path has been processed by [`AssetProcessor`].
///
/// [`AssetProcessor`]: crate::processor::AssetProcessor   
pub struct ProcessorGatedReader {
    reader: Box<dyn ErasedAssetReader>,
    source: AssetSourceId<'static>,
    processor_data: Arc<AssetProcessorData>,
}

impl ProcessorGatedReader {
    /// Creates a new [`ProcessorGatedReader`].
    pub fn new(
        source: AssetSourceId<'static>,
        reader: Box<dyn ErasedAssetReader>,
        processor_data: Arc<AssetProcessorData>,
    ) -> Self {
        Self {
            source,
            processor_data,
            reader,
        }
    }

    /// Gets a "transaction lock" that can be used to ensure no writes to asset or asset meta occur
    /// while it is held.
    async fn get_transaction_lock(
        &self,
        path: &AssetPath<'static>,
    ) -> Result<RwLockReadGuardArc<()>, AssetReaderError> {
        let infos = self.processor_data.asset_infos.read().await;
        let info = infos
            .get(path)
            .ok_or_else(|| AssetReaderError::NotFound(path.path().to_owned()))?;
        Ok(info.file_transaction_lock.read_arc().await)
    }
}

impl AssetReader for ProcessorGatedReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let asset_path = AssetPath::from(path.to_path_buf()).with_source(self.source.clone());
        trace!("Waiting for processing to finish before reading {asset_path}");
        let process_result = self
            .processor_data
            .wait_until_processed(asset_path.clone())
            .await;
        match process_result {
            ProcessStatus::Processed => {}
            ProcessStatus::Failed | ProcessStatus::NonExistent => {
                return Err(AssetReaderError::NotFound(path.to_owned()));
            }
        }
        trace!("Processing finished with {asset_path}, reading {process_result:?}",);
        let lock = self.get_transaction_lock(&asset_path).await?;
        let asset_reader = self.reader.read(path).await?;
        let reader: Box<Reader<'a>> = Box::new(TransactionLockedReader::new(asset_reader, lock));
        Ok(reader)
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let asset_path = AssetPath::from(path.to_path_buf()).with_source(self.source.clone());
        trace!("Waiting for processing to finish before reading meta for {asset_path}",);
        let process_result = self
            .processor_data
            .wait_until_processed(asset_path.clone())
            .await;
        match process_result {
            ProcessStatus::Processed => {}
            ProcessStatus::Failed | ProcessStatus::NonExistent => {
                return Err(AssetReaderError::NotFound(path.to_owned()));
            }
        }
        trace!("Processing finished with {process_result:?}, reading meta for {asset_path}",);
        let lock = self.get_transaction_lock(&asset_path).await?;
        let meta_reader = self.reader.read_meta(path).await?;
        let reader: Box<Reader<'a>> = Box::new(TransactionLockedReader::new(meta_reader, lock));
        Ok(reader)
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        trace!(
            "Waiting for processing to finish before reading directory {:?}",
            path
        );
        self.processor_data.wait_until_finished().await;
        trace!("Processing finished, reading directory {:?}", path);
        let result = self.reader.read_directory(path).await?;
        Ok(result)
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        trace!(
            "Waiting for processing to finish before reading directory {:?}",
            path
        );
        self.processor_data.wait_until_finished().await;
        trace!("Processing finished, getting directory status {:?}", path);
        let result = self.reader.is_directory(path).await?;
        Ok(result)
    }
}

/// An [`AsyncRead`] impl that will hold its asset's transaction lock until [`TransactionLockedReader`] is dropped.
pub struct TransactionLockedReader<'a> {
    reader: Box<Reader<'a>>,
    _file_transaction_lock: RwLockReadGuardArc<()>,
}

impl<'a> TransactionLockedReader<'a> {
    fn new(reader: Box<Reader<'a>>, file_transaction_lock: RwLockReadGuardArc<()>) -> Self {
        Self {
            reader,
            _file_transaction_lock: file_transaction_lock,
        }
    }
}

impl<'a> AsyncRead for TransactionLockedReader<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<futures_io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl<'a> AsyncSeek for TransactionLockedReader<'a> {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        Pin::new(&mut self.reader).poll_seek(cx, pos)
    }
}
