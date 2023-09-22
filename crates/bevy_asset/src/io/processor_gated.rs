use crate::{
    io::{AssetReader, AssetReaderError, PathStream, Reader},
    processor::{AssetProcessorData, ProcessStatus},
    AssetPath,
};
use anyhow::Result;
use async_lock::RwLockReadGuardArc;
use bevy_log::trace;
use bevy_utils::BoxedFuture;
use futures_io::AsyncRead;
use std::{path::Path, pin::Pin, sync::Arc};

/// An [`AssetReader`] that will prevent asset (and asset metadata) read futures from returning for a
/// given path until that path has been processed by [`AssetProcessor`].
///
/// [`AssetProcessor`]: crate::processor::AssetProcessor   
pub struct ProcessorGatedReader {
    reader: Box<dyn AssetReader>,
    processor_data: Arc<AssetProcessorData>,
}

impl ProcessorGatedReader {
    /// Creates a new [`ProcessorGatedReader`].
    pub fn new(reader: Box<dyn AssetReader>, processor_data: Arc<AssetProcessorData>) -> Self {
        Self {
            processor_data,
            reader,
        }
    }

    /// Gets a "transaction lock" that can be used to ensure no writes to asset or asset meta occur
    /// while it is held.
    async fn get_transaction_lock(
        &self,
        path: &Path,
    ) -> Result<RwLockReadGuardArc<()>, AssetReaderError> {
        let infos = self.processor_data.asset_infos.read().await;
        let info = infos
            .get(&AssetPath::from_path(path.to_path_buf()))
            .ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;
        Ok(info.file_transaction_lock.read_arc().await)
    }
}

impl AssetReader for ProcessorGatedReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            trace!("Waiting for processing to finish before reading {:?}", path);
            let process_result = self.processor_data.wait_until_processed(path).await;
            match process_result {
                ProcessStatus::Processed => {}
                ProcessStatus::Failed | ProcessStatus::NonExistent => {
                    return Err(AssetReaderError::NotFound(path.to_owned()))
                }
            }
            trace!(
                "Processing finished with {:?}, reading {:?}",
                process_result,
                path
            );
            let lock = self.get_transaction_lock(path).await?;
            let asset_reader = self.reader.read(path).await?;
            let reader: Box<Reader<'a>> =
                Box::new(TransactionLockedReader::new(asset_reader, lock));
            Ok(reader)
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            trace!(
                "Waiting for processing to finish before reading meta {:?}",
                path
            );
            let process_result = self.processor_data.wait_until_processed(path).await;
            match process_result {
                ProcessStatus::Processed => {}
                ProcessStatus::Failed | ProcessStatus::NonExistent => {
                    return Err(AssetReaderError::NotFound(path.to_owned()));
                }
            }
            trace!(
                "Processing finished with {:?}, reading meta {:?}",
                process_result,
                path
            );
            let lock = self.get_transaction_lock(path).await?;
            let meta_reader = self.reader.read_meta(path).await?;
            let reader: Box<Reader<'a>> = Box::new(TransactionLockedReader::new(meta_reader, lock));
            Ok(reader)
        })
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move {
            trace!(
                "Waiting for processing to finish before reading directory {:?}",
                path
            );
            self.processor_data.wait_until_finished().await;
            trace!("Processing finished, reading directory {:?}", path);
            let result = self.reader.read_directory(path).await?;
            Ok(result)
        })
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<bool, AssetReaderError>> {
        Box::pin(async move {
            trace!(
                "Waiting for processing to finish before reading directory {:?}",
                path
            );
            self.processor_data.wait_until_finished().await;
            trace!("Processing finished, getting directory status {:?}", path);
            let result = self.reader.is_directory(path).await?;
            Ok(result)
        })
    }

    fn watch_for_changes(
        &self,
        event_sender: crossbeam_channel::Sender<super::AssetSourceEvent>,
    ) -> Option<Box<dyn super::AssetWatcher>> {
        self.reader.watch_for_changes(event_sender)
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
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<futures_io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}
