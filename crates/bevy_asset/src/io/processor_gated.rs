use crate::{
    io::{AssetReader, AssetReaderError, PathStream, Reader},
    processor::{AssetProcessorData, ProcessStatus},
    AssetPath,
};
use anyhow::Result;
use bevy_log::trace;
use bevy_utils::BoxedFuture;
use futures_io::AsyncRead;
use parking_lot::lock_api::ArcRwLockReadGuard;
use std::{path::Path, pin::Pin, sync::Arc};

pub struct ProcessorGatedReader {
    reader: Box<dyn AssetReader>,
    processor_data: Arc<AssetProcessorData>,
}

impl ProcessorGatedReader {
    pub fn new(reader: Box<dyn AssetReader>, processor_data: Arc<AssetProcessorData>) -> Self {
        Self {
            processor_data,
            reader,
        }
    }
    async fn get_transaction_lock(
        &self,
        path: &Path,
    ) -> Result<ArcRwLockReadGuard<parking_lot::RawRwLock, ()>, AssetReaderError> {
        let infos = self.processor_data.asset_infos.read().await;
        let info = infos
            .get(&AssetPath::new(path.to_owned(), None))
            .ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;
        Ok(info.file_transaction_lock.read_arc())
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

pub struct TransactionLockedReader<'a> {
    reader: Box<Reader<'a>>,
    _file_transaction_lock: ArcRwLockReadGuard<parking_lot::RawRwLock, ()>,
}

impl<'a> TransactionLockedReader<'a> {
    fn new(
        reader: Box<Reader<'a>>,
        file_transaction_lock: ArcRwLockReadGuard<parking_lot::RawRwLock, ()>,
    ) -> Self {
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
