use crate::{
    io::{
        AssetReader, AssetReaderError, AssetSourceId, PathStream, Reader, ReaderNotSeekableError,
        SeekableReader,
    },
    processor::{ProcessStatus, ProcessingState},
    AssetPath,
};
use alloc::{borrow::ToOwned, boxed::Box, sync::Arc, vec::Vec};
use async_lock::RwLockReadGuardArc;
use core::{pin::Pin, task::Poll};
use futures_io::AsyncRead;
use std::path::Path;
use tracing::trace;

use super::ErasedAssetReader;

/// An [`AssetReader`] that will prevent asset (and asset metadata) read futures from returning for a
/// given path until that path has been processed by [`AssetProcessor`].
///
/// [`AssetProcessor`]: crate::processor::AssetProcessor
pub(crate) struct ProcessorGatedReader {
    reader: Arc<dyn ErasedAssetReader>,
    source: AssetSourceId<'static>,
    processing_state: Arc<ProcessingState>,
}

impl ProcessorGatedReader {
    /// Creates a new [`ProcessorGatedReader`].
    pub(crate) fn new(
        source: AssetSourceId<'static>,
        reader: Arc<dyn ErasedAssetReader>,
        processing_state: Arc<ProcessingState>,
    ) -> Self {
        Self {
            source,
            reader,
            processing_state,
        }
    }
}

impl AssetReader for ProcessorGatedReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let asset_path = AssetPath::from(path.to_path_buf()).with_source(self.source.clone());
        trace!("Waiting for processing to finish before reading {asset_path}");
        let process_result = self
            .processing_state
            .wait_until_processed(asset_path.clone())
            .await;
        match process_result {
            ProcessStatus::Processed => {}
            ProcessStatus::Failed | ProcessStatus::NonExistent => {
                return Err(AssetReaderError::NotFound(path.to_owned()));
            }
        }
        trace!("Processing finished with {asset_path}, reading {process_result:?}",);
        let lock = self
            .processing_state
            .get_transaction_lock(&asset_path)
            .await?;
        let asset_reader = self.reader.read(path).await?;
        let reader = TransactionLockedReader::new(asset_reader, lock);
        Ok(reader)
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let asset_path = AssetPath::from(path.to_path_buf()).with_source(self.source.clone());
        trace!("Waiting for processing to finish before reading meta for {asset_path}",);
        let process_result = self
            .processing_state
            .wait_until_processed(asset_path.clone())
            .await;
        match process_result {
            ProcessStatus::Processed => {}
            ProcessStatus::Failed | ProcessStatus::NonExistent => {
                return Err(AssetReaderError::NotFound(path.to_owned()));
            }
        }
        trace!("Processing finished with {process_result:?}, reading meta for {asset_path}",);
        let lock = self
            .processing_state
            .get_transaction_lock(&asset_path)
            .await?;
        let meta_reader = self.reader.read_meta(path).await?;
        let reader = TransactionLockedReader::new(meta_reader, lock);
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
        self.processing_state.wait_until_finished().await;
        trace!("Processing finished, reading directory {:?}", path);
        let result = self.reader.read_directory(path).await?;
        Ok(result)
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        trace!(
            "Waiting for processing to finish before reading directory {:?}",
            path
        );
        self.processing_state.wait_until_finished().await;
        trace!("Processing finished, getting directory status {:?}", path);
        let result = self.reader.is_directory(path).await?;
        Ok(result)
    }
}

/// An [`AsyncRead`] impl that will hold its asset's transaction lock until [`TransactionLockedReader`] is dropped.
pub struct TransactionLockedReader<'a> {
    reader: Box<dyn Reader + 'a>,
    _file_transaction_lock: RwLockReadGuardArc<()>,
}

impl<'a> TransactionLockedReader<'a> {
    fn new(reader: Box<dyn Reader + 'a>, file_transaction_lock: RwLockReadGuardArc<()>) -> Self {
        Self {
            reader,
            _file_transaction_lock: file_transaction_lock,
        }
    }
}

impl AsyncRead for TransactionLockedReader<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl Reader for TransactionLockedReader<'_> {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> stackfuture::StackFuture<'a, std::io::Result<usize>, { super::STACK_FUTURE_SIZE }> {
        self.reader.read_to_end(buf)
    }

    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        self.reader.seekable()
    }
}
