//! Tools for waiting for the [`AssetProcessor`] before loading an [`Asset`].
//!
//! [`AssetProcessor`]: crate::processor::AssetProcessor
//! [`Asset`]: crate::Asset

use crate::{
    io::{
        AssetReader, AssetReaderError, AssetSourceId, MetaAndAssetReaders, PathStream, Reader,
        ReaderNotSeekableError, SeekableReader,
    },
    processor::{ProcessStatus, ProcessingState},
    AssetPath,
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use async_lock::RwLockReadGuardArc;
use atomicow::CowArc;
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
    async fn read<'a>(
        &'a self,
        path: CowArc<'a, Path>,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        let asset_path = AssetPath::from(path.to_path_buf()).with_source(self.source.clone());
        trace!("Waiting for processing to finish before reading {asset_path}");
        let process_result = self
            .processing_state
            .wait_until_processed(asset_path.clone())
            .await;
        match process_result {
            ProcessStatus::Processed => {}
            ProcessStatus::Failed | ProcessStatus::NonExistent => {
                return Err(AssetReaderError::NotFound(path.to_path_buf()));
            }
        }
        trace!("Processing finished with {asset_path}, reading {process_result:?}",);
        let lock = self
            .processing_state
            .get_transaction_lock(&asset_path)
            .await?;
        let asset_reader = self.reader.read(path).await?;
        let reader = TransactionLockedReader::new(asset_reader, Arc::new(lock));
        Ok(reader)
    }

    async fn read_meta<'a>(
        &'a self,
        path: CowArc<'a, Path>,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        let asset_path = AssetPath::from(path.to_path_buf()).with_source(self.source.clone());
        trace!("Waiting for processing to finish before reading meta for {asset_path}",);
        let process_result = self
            .processing_state
            .wait_until_processed(asset_path.clone())
            .await;
        match process_result {
            ProcessStatus::Processed => {}
            ProcessStatus::Failed | ProcessStatus::NonExistent => {
                return Err(AssetReaderError::NotFound(path.to_path_buf()));
            }
        }
        trace!("Processing finished with {process_result:?}, reading meta for {asset_path}",);
        let lock = self
            .processing_state
            .get_transaction_lock(&asset_path)
            .await?;
        let meta_reader = self.reader.read_meta(path).await?;
        let reader = TransactionLockedReader::new(meta_reader, Arc::new(lock));
        Ok(reader)
    }

    async fn read_meta_and_asset<'a>(&'a self, path: CowArc<'a, Path>) -> MetaAndAssetReaders<'a> {
        let asset_path = AssetPath::from(path.to_path_buf()).with_source(self.source.clone());
        trace!("Waiting for processing to finish before reading meta and asset for {asset_path}");
        let process_result = self
            .processing_state
            .wait_until_processed(asset_path.clone())
            .await;
        match process_result {
            ProcessStatus::Processed => {}
            ProcessStatus::Failed | ProcessStatus::NonExistent => {
                let err = AssetReaderError::NotFound(path.to_path_buf());
                return (Err(err.clone()), Err(err));
            }
        }
        trace!(
            "Processing finished with {process_result:?}, reading meta and asset for {asset_path}"
        );
        // Take the transaction lock once and share it between both readers. `async_lock` makes new
        // readers wait behind a waiting writer, so locking once per reader can deadlock: if the
        // processor queues a write between the two reads, the second read waits for the writer,
        // which waits for the first read.
        let lock = match self
            .processing_state
            .get_transaction_lock(&asset_path)
            .await
        {
            Ok(lock) => Arc::new(lock),
            Err(err) => return (Err(err.clone()), Err(err)),
        };
        let meta_reader = self.reader.read_meta(path.clone()).await.map(|reader| {
            Box::new(TransactionLockedReader::new(reader, lock.clone())) as Box<dyn Reader + 'a>
        });
        let reader = self.reader.read(path).await.map(|reader| {
            Box::new(TransactionLockedReader::new(reader, lock)) as Box<dyn Reader + 'a>
        });
        (meta_reader, reader)
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
///
/// The guard can be shared by several readers (for example an asset and its meta file). The lock is
/// only released once all of them have been dropped.
pub struct TransactionLockedReader<'a> {
    reader: Box<dyn Reader + 'a>,
    _file_transaction_lock: Arc<RwLockReadGuardArc<()>>,
}

impl<'a> TransactionLockedReader<'a> {
    fn new(
        reader: Box<dyn Reader + 'a>,
        file_transaction_lock: Arc<RwLockReadGuardArc<()>>,
    ) -> Self {
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
