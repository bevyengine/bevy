//! Asset processing in Bevy is a framework for automatically transforming artist-authored assets into the format that best suits the needs of your particular game.
//!
//! You can think of the asset processing system as a "build system" for assets.
//! When an artist adds a new asset to the project or an asset is changed (assuming asset hot reloading is enabled), the asset processing system will automatically perform the specified processing steps on the asset.
//! This can include things like creating lightmaps for baked lighting, compressing a `.wav` file to an `.ogg`, or generating mipmaps for a texture.
//!
//! Its core values are:
//!
//! 1. Automatic: new and changed assets should be ready to use in-game without requiring any manual conversion or cleanup steps.
//! 2. Configurable: every game has its own needs, and a high level of transparency and control is required.
//! 3. Lossless: the original asset should always be preserved, ensuring artists can make changes later.
//! 4. Deterministic: performing the same processing steps on the same asset should (generally) produce the exact same result. In cases where this doesn't make sense (steps that involve a degree of randomness or uncertainty), the results across runs should be "acceptably similar", as they will be generated once for a given set of inputs and cached.
//!
//! Taken together, this means that the original asset plus the processing steps should be enough to regenerate the final asset.
//! While it may be possible to manually edit the final asset, this should be discouraged.
//! Final post-processed assets should generally not be version-controlled, except to save developer time when recomputing heavy asset processing steps.
//!
//! # Usage
//!
//! Asset processing can be enabled or disabled in [`AssetPlugin`](crate::AssetPlugin) by setting the [`AssetMode`](crate::AssetMode).\
//! Enable Bevy's `file_watcher` feature to automatically watch for changes to assets and reprocess them.
//!
//! To register a new asset processor, use [`AssetProcessor::register_processor`].
//! To set the default asset processor for a given extension, use [`AssetProcessor::set_default_processor`].
//! In most cases, these methods will be called directly on [`App`](bevy_app::App) using the [`AssetApp`](crate::AssetApp) extension trait.
//!
//! If a default asset processor is set, assets with a matching extension will be processed using that processor before loading.
//!
//! For an end-to-end example, check out the examples in the [`examples/asset/processing`](https://github.com/bevyengine/bevy/tree/latest/examples/asset/processing) directory of the Bevy repository.
//!
//!  # Defining asset processors
//!
//! Bevy provides two different ways to define new asset processors:
//!
//! - [`LoadTransformAndSave`] + [`AssetTransformer`](crate::transformer::AssetTransformer): a high-level API for loading, transforming, and saving assets.
//! - [`Process`]: a flexible low-level API for processing assets in arbitrary ways.
//!
//! In most cases, [`LoadTransformAndSave`] should be sufficient.

mod log;
mod process;

pub use log::*;
pub use process::*;

use crate::{
    io::{
        AssetReaderError, AssetSource, AssetSourceBuilders, AssetSourceEvent, AssetSourceId,
        AssetSources, AssetWriterError, ErasedAssetReader, ErasedAssetWriter,
        MissingAssetSourceError,
    },
    meta::{
        get_asset_hash, get_full_asset_hash, AssetAction, AssetActionMinimal, AssetHash, AssetMeta,
        AssetMetaDyn, AssetMetaMinimal, ProcessedInfo, ProcessedInfoMinimal,
    },
    AssetLoadError, AssetMetaCheck, AssetPath, AssetServer, AssetServerMode, DeserializeMetaError,
    MissingAssetLoaderForExtensionError, UnapprovedPathMode, WriteDefaultMetaError,
};
use alloc::{borrow::ToOwned, boxed::Box, collections::VecDeque, sync::Arc, vec, vec::Vec};
use bevy_ecs::prelude::*;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_tasks::IoTaskPool;
use futures_io::ErrorKind;
use futures_lite::{AsyncReadExt, AsyncWriteExt, StreamExt};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, error, trace, warn};

#[cfg(feature = "trace")]
use {
    alloc::string::ToString,
    bevy_tasks::ConditionalSendFuture,
    tracing::{info_span, instrument::Instrument},
};

/// A "background" asset processor that reads asset values from a source [`AssetSource`] (which corresponds to an [`AssetReader`](crate::io::AssetReader) / [`AssetWriter`](crate::io::AssetWriter) pair),
/// processes them in some way, and writes them to a destination [`AssetSource`].
///
/// This will create .meta files (a human-editable serialized form of [`AssetMeta`]) in the source [`AssetSource`] for assets that
/// that can be loaded and/or processed. This enables developers to configure how each asset should be loaded and/or processed.
///
/// [`AssetProcessor`] can be run in the background while a Bevy App is running. Changes to assets will be automatically detected and hot-reloaded.
///
/// Assets will only be re-processed if they have been changed. A hash of each asset source is stored in the metadata of the processed version of the
/// asset, which is used to determine if the asset source has actually changed.
///
/// A [`ProcessorTransactionLog`] is produced, which uses "write-ahead logging" to make the [`AssetProcessor`] crash and failure resistant. If a failed/unfinished
/// transaction from a previous run is detected, the affected asset(s) will be re-processed.
///
/// [`AssetProcessor`] can be cloned. It is backed by an [`Arc`] so clones will share state. Clones can be freely used in parallel.
#[derive(Resource, Clone)]
pub struct AssetProcessor {
    server: AssetServer,
    pub(crate) data: Arc<AssetProcessorData>,
}

/// Internal data stored inside an [`AssetProcessor`].
pub struct AssetProcessorData {
    pub(crate) asset_infos: async_lock::RwLock<ProcessorAssetInfos>,
    log: async_lock::RwLock<Option<ProcessorTransactionLog>>,
    processors: RwLock<HashMap<&'static str, Arc<dyn ErasedProcessor>>>,
    /// Default processors for file extensions
    default_processors: RwLock<HashMap<Box<str>, &'static str>>,
    state: async_lock::RwLock<ProcessorState>,
    sources: AssetSources,
    initialized_sender: async_broadcast::Sender<()>,
    initialized_receiver: async_broadcast::Receiver<()>,
    finished_sender: async_broadcast::Sender<()>,
    finished_receiver: async_broadcast::Receiver<()>,
}

impl AssetProcessor {
    /// Creates a new [`AssetProcessor`] instance.
    pub fn new(source: &mut AssetSourceBuilders) -> Self {
        let data = Arc::new(AssetProcessorData::new(source.build_sources(true, false)));
        // The asset processor uses its own asset server with its own id space
        let mut sources = source.build_sources(false, false);
        sources.gate_on_processor(data.clone());
        let server = AssetServer::new_with_meta_check(
            sources,
            AssetServerMode::Processed,
            AssetMetaCheck::Always,
            false,
            UnapprovedPathMode::default(),
        );
        Self { server, data }
    }

    /// Gets a reference to the [`Arc`] containing the [`AssetProcessorData`].
    pub fn data(&self) -> &Arc<AssetProcessorData> {
        &self.data
    }

    /// The "internal" [`AssetServer`] used by the [`AssetProcessor`]. This is _separate_ from the asset processor used by
    /// the main App. It has different processor-specific configuration and a different ID space.
    pub fn server(&self) -> &AssetServer {
        &self.server
    }

    async fn set_state(&self, state: ProcessorState) {
        let mut state_guard = self.data.state.write().await;
        let last_state = *state_guard;
        *state_guard = state;
        if last_state != ProcessorState::Finished && state == ProcessorState::Finished {
            self.data.finished_sender.broadcast(()).await.unwrap();
        } else if last_state != ProcessorState::Processing && state == ProcessorState::Processing {
            self.data.initialized_sender.broadcast(()).await.unwrap();
        }
    }

    /// Retrieves the current [`ProcessorState`]
    pub async fn get_state(&self) -> ProcessorState {
        *self.data.state.read().await
    }

    /// Retrieves the [`AssetSource`] for this processor
    #[inline]
    pub fn get_source<'a>(
        &self,
        id: impl Into<AssetSourceId<'a>>,
    ) -> Result<&AssetSource, MissingAssetSourceError> {
        self.data.sources.get(id.into())
    }

    #[inline]
    pub fn sources(&self) -> &AssetSources {
        &self.data.sources
    }

    /// Logs an unrecoverable error. On the next run of the processor, all assets will be regenerated. This should only be used as a last resort.
    /// Every call to this should be considered with scrutiny and ideally replaced with something more granular.
    async fn log_unrecoverable(&self) {
        let mut log = self.data.log.write().await;
        let log = log.as_mut().unwrap();
        log.unrecoverable().await.unwrap();
    }

    /// Logs the start of an asset being processed. If this is not followed at some point in the log by a closing [`AssetProcessor::log_end_processing`],
    /// in the next run of the processor the asset processing will be considered "incomplete" and it will be reprocessed.
    async fn log_begin_processing(&self, path: &AssetPath<'_>) {
        let mut log = self.data.log.write().await;
        let log = log.as_mut().unwrap();
        log.begin_processing(path).await.unwrap();
    }

    /// Logs the end of an asset being successfully processed. See [`AssetProcessor::log_begin_processing`].
    async fn log_end_processing(&self, path: &AssetPath<'_>) {
        let mut log = self.data.log.write().await;
        let log = log.as_mut().unwrap();
        log.end_processing(path).await.unwrap();
    }

    /// Starts the processor in a background thread.
    pub fn start(_processor: Res<Self>) {
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        error!("Cannot run AssetProcessor in single threaded mode (or Wasm) yet.");
        #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
        {
            let processor = _processor.clone();
            std::thread::spawn(move || {
                processor.process_assets();
                bevy_tasks::block_on(processor.listen_for_source_change_events());
            });
        }
    }

    /// Processes all assets. This will:
    /// * For each "processed [`AssetSource`]:
    /// * Scan the [`ProcessorTransactionLog`] and recover from any failures detected
    /// * Scan the processed [`AssetReader`](crate::io::AssetReader) to build the current view of
    ///   already processed assets.
    /// * Scan the unprocessed [`AssetReader`](crate::io::AssetReader) and remove any final
    ///   processed assets that are invalid or no longer exist.
    /// * For each asset in the unprocessed [`AssetReader`](crate::io::AssetReader), kick off a new
    ///   "process job", which will process the asset
    ///   (if the latest version of the asset has not been processed).
    #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
    pub fn process_assets(&self) {
        let start_time = std::time::Instant::now();
        debug!("Processing Assets");
        IoTaskPool::get().scope(|scope| {
            scope.spawn(async move {
                self.initialize().await.unwrap();
                for source in self.sources().iter_processed() {
                    self.process_assets_internal(scope, source, PathBuf::from(""))
                        .await
                        .unwrap();
                }
            });
        });
        // This must happen _after_ the scope resolves or it will happen "too early"
        // Don't move this into the async scope above! process_assets is a blocking/sync function this is fine
        bevy_tasks::block_on(self.finish_processing_assets());
        let end_time = std::time::Instant::now();
        debug!("Processing finished in {:?}", end_time - start_time);
    }

    /// Listens for changes to assets in the source [`AssetSource`] and update state accordingly.
    // PERF: parallelize change event processing
    pub async fn listen_for_source_change_events(&self) {
        debug!("Listening for changes to source assets");
        loop {
            let mut started_processing = false;

            for source in self.data.sources.iter_processed() {
                if let Some(receiver) = source.event_receiver() {
                    for event in receiver.try_iter() {
                        if !started_processing {
                            self.set_state(ProcessorState::Processing).await;
                            started_processing = true;
                        }

                        self.handle_asset_source_event(source, event).await;
                    }
                }
            }

            if started_processing {
                self.finish_processing_assets().await;
            }
        }
    }

    /// Writes the default meta file for the provided `path`.
    ///
    /// This function generates the appropriate meta file to process `path` with the default
    /// processor. If there is no default processor, it falls back to the default loader.
    ///
    /// Note if there is already a meta file for `path`, this function returns
    /// `Err(WriteDefaultMetaError::MetaAlreadyExists)`.
    pub async fn write_default_meta_file_for_path(
        &self,
        path: impl Into<AssetPath<'_>>,
    ) -> Result<(), WriteDefaultMetaError> {
        let path = path.into();
        let Some(processor) = path
            .get_full_extension()
            .and_then(|extension| self.get_default_processor(&extension))
        else {
            return self
                .server
                .write_default_loader_meta_file_for_path(path)
                .await;
        };

        let meta = processor.default_meta();
        let serialized_meta = meta.serialize();

        let source = self.get_source(path.source())?;

        // Note: we get the reader rather than the processed reader, since we want to write the meta
        // file for the unprocessed version of that asset (so it will be processed by the default
        // processor).
        let reader = source.reader();
        match reader.read_meta_bytes(path.path()).await {
            Ok(_) => return Err(WriteDefaultMetaError::MetaAlreadyExists),
            Err(AssetReaderError::NotFound(_)) => {
                // The meta file couldn't be found so just fall through.
            }
            Err(AssetReaderError::Io(err)) => {
                return Err(WriteDefaultMetaError::IoErrorFromExistingMetaCheck(err))
            }
            Err(AssetReaderError::HttpError(err)) => {
                return Err(WriteDefaultMetaError::HttpErrorFromExistingMetaCheck(err))
            }
        }

        let writer = source.writer()?;
        writer
            .write_meta_bytes(path.path(), &serialized_meta)
            .await?;

        Ok(())
    }

    async fn handle_asset_source_event(&self, source: &AssetSource, event: AssetSourceEvent) {
        trace!("{event:?}");
        match event {
            AssetSourceEvent::AddedAsset(path)
            | AssetSourceEvent::AddedMeta(path)
            | AssetSourceEvent::ModifiedAsset(path)
            | AssetSourceEvent::ModifiedMeta(path) => {
                self.process_asset(source, path).await;
            }
            AssetSourceEvent::RemovedAsset(path) => {
                self.handle_removed_asset(source, path).await;
            }
            AssetSourceEvent::RemovedMeta(path) => {
                self.handle_removed_meta(source, path).await;
            }
            AssetSourceEvent::AddedFolder(path) => {
                self.handle_added_folder(source, path).await;
            }
            // NOTE: As a heads up for future devs: this event shouldn't be run in parallel with other events that might
            // touch this folder (ex: the folder might be re-created with new assets). Clean up the old state first.
            // Currently this event handler is not parallel, but it could be (and likely should be) in the future.
            AssetSourceEvent::RemovedFolder(path) => {
                self.handle_removed_folder(source, &path).await;
            }
            AssetSourceEvent::RenamedAsset { old, new } => {
                // If there was a rename event, but the path hasn't changed, this asset might need reprocessing.
                // Sometimes this event is returned when an asset is moved "back" into the asset folder
                if old == new {
                    self.process_asset(source, new).await;
                } else {
                    self.handle_renamed_asset(source, old, new).await;
                }
            }
            AssetSourceEvent::RenamedMeta { old, new } => {
                // If there was a rename event, but the path hasn't changed, this asset meta might need reprocessing.
                // Sometimes this event is returned when an asset meta is moved "back" into the asset folder
                if old == new {
                    self.process_asset(source, new).await;
                } else {
                    debug!("Meta renamed from {old:?} to {new:?}");
                    let mut infos = self.data.asset_infos.write().await;
                    // Renaming meta should not assume that an asset has also been renamed. Check both old and new assets to see
                    // if they should be re-imported (and/or have new meta generated)
                    let new_asset_path = AssetPath::from(new).with_source(source.id());
                    let old_asset_path = AssetPath::from(old).with_source(source.id());
                    infos.check_reprocess_queue.push_back(old_asset_path);
                    infos.check_reprocess_queue.push_back(new_asset_path);
                }
            }
            AssetSourceEvent::RenamedFolder { old, new } => {
                // If there was a rename event, but the path hasn't changed, this asset folder might need reprocessing.
                // Sometimes this event is returned when an asset meta is moved "back" into the asset folder
                if old == new {
                    self.handle_added_folder(source, new).await;
                } else {
                    // PERF: this reprocesses everything in the moved folder. this is not necessary in most cases, but
                    // requires some nuance when it comes to path handling.
                    self.handle_removed_folder(source, &old).await;
                    self.handle_added_folder(source, new).await;
                }
            }
            AssetSourceEvent::RemovedUnknown { path, is_meta } => {
                let processed_reader = source.processed_reader().unwrap();
                match processed_reader.is_directory(&path).await {
                    Ok(is_directory) => {
                        if is_directory {
                            self.handle_removed_folder(source, &path).await;
                        } else if is_meta {
                            self.handle_removed_meta(source, path).await;
                        } else {
                            self.handle_removed_asset(source, path).await;
                        }
                    }
                    Err(err) => {
                        match err {
                            AssetReaderError::NotFound(_) => {
                                // if the path is not found, a processed version does not exist
                            }
                            AssetReaderError::Io(err) => {
                                error!(
                                    "Path '{}' was removed, but the destination reader could not determine if it \
                                    was a folder or a file due to the following error: {err}",
                                    AssetPath::from_path(&path).with_source(source.id())
                                );
                            }
                            AssetReaderError::HttpError(status) => {
                                error!(
                                    "Path '{}' was removed, but the destination reader could not determine if it \
                                    was a folder or a file due to receiving an unexpected HTTP Status {status}",
                                    AssetPath::from_path(&path).with_source(source.id())
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    async fn handle_added_folder(&self, source: &AssetSource, path: PathBuf) {
        debug!(
            "Folder {} was added. Attempting to re-process",
            AssetPath::from_path(&path).with_source(source.id())
        );
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        error!("AddFolder event cannot be handled in single threaded mode (or Wasm) yet.");
        #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
        IoTaskPool::get().scope(|scope| {
            scope.spawn(async move {
                self.process_assets_internal(scope, source, path)
                    .await
                    .unwrap();
            });
        });
    }

    /// Responds to a removed meta event by reprocessing the asset at the given path.
    async fn handle_removed_meta(&self, source: &AssetSource, path: PathBuf) {
        // If meta was removed, we might need to regenerate it.
        // Likewise, the user might be manually re-adding the asset.
        // Therefore, we shouldn't automatically delete the asset ... that is a
        // user-initiated action.
        debug!(
            "Meta for asset {} was removed. Attempting to re-process",
            AssetPath::from_path(&path).with_source(source.id())
        );
        self.process_asset(source, path).await;
    }

    /// Removes all processed assets stored at the given path (respecting transactionality), then removes the folder itself.
    async fn handle_removed_folder(&self, source: &AssetSource, path: &Path) {
        debug!(
            "Removing folder {} because source was removed",
            path.display()
        );
        let processed_reader = source.processed_reader().unwrap();
        match processed_reader.read_directory(path).await {
            Ok(mut path_stream) => {
                while let Some(child_path) = path_stream.next().await {
                    self.handle_removed_asset(source, child_path).await;
                }
            }
            Err(err) => match err {
                AssetReaderError::NotFound(_err) => {
                    // The processed folder does not exist. No need to update anything
                }
                AssetReaderError::HttpError(status) => {
                    self.log_unrecoverable().await;
                    error!(
                        "Unrecoverable Error: Failed to read the processed assets at {path:?} in order to remove assets that no longer exist \
                        in the source directory. Restart the asset processor to fully reprocess assets. HTTP Status Code {status}"
                    );
                }
                AssetReaderError::Io(err) => {
                    self.log_unrecoverable().await;
                    error!(
                        "Unrecoverable Error: Failed to read the processed assets at {path:?} in order to remove assets that no longer exist \
                        in the source directory. Restart the asset processor to fully reprocess assets. Error: {err}"
                    );
                }
            },
        }
        let processed_writer = source.processed_writer().unwrap();
        if let Err(err) = processed_writer.remove_directory(path).await {
            match err {
                AssetWriterError::Io(err) => {
                    // we can ignore NotFound because if the "final" file in a folder was removed
                    // then we automatically clean up this folder
                    if err.kind() != ErrorKind::NotFound {
                        let asset_path = AssetPath::from_path(path).with_source(source.id());
                        error!("Failed to remove destination folder that no longer exists in {asset_path}: {err}");
                    }
                }
            }
        }
    }

    /// Removes the processed version of an asset and associated in-memory metadata. This will block until all existing reads/writes to the
    /// asset have finished, thanks to the `file_transaction_lock`.
    async fn handle_removed_asset(&self, source: &AssetSource, path: PathBuf) {
        let asset_path = AssetPath::from(path).with_source(source.id());
        debug!("Removing processed {asset_path} because source was removed");
        let mut infos = self.data.asset_infos.write().await;
        if let Some(info) = infos.get(&asset_path) {
            // we must wait for uncontested write access to the asset source to ensure existing readers / writers
            // can finish their operations
            let _write_lock = info.file_transaction_lock.write();
            self.remove_processed_asset_and_meta(source, asset_path.path())
                .await;
        }
        infos.remove(&asset_path).await;
    }

    /// Handles a renamed source asset by moving its processed results to the new location and updating in-memory paths + metadata.
    /// This will cause direct path dependencies to break.
    async fn handle_renamed_asset(&self, source: &AssetSource, old: PathBuf, new: PathBuf) {
        let mut infos = self.data.asset_infos.write().await;
        let old = AssetPath::from(old).with_source(source.id());
        let new = AssetPath::from(new).with_source(source.id());
        let processed_writer = source.processed_writer().unwrap();
        if let Some(info) = infos.get(&old) {
            // we must wait for uncontested write access to the asset source to ensure existing readers / writers
            // can finish their operations
            let _write_lock = info.file_transaction_lock.write();
            processed_writer
                .rename(old.path(), new.path())
                .await
                .unwrap();
            processed_writer
                .rename_meta(old.path(), new.path())
                .await
                .unwrap();
        }
        infos.rename(&old, &new).await;
    }

    async fn finish_processing_assets(&self) {
        self.try_reprocessing_queued().await;
        // clean up metadata in asset server
        self.server.data.infos.write().consume_handle_drop_events();
        self.set_state(ProcessorState::Finished).await;
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
    async fn process_assets_internal<'scope>(
        &'scope self,
        scope: &'scope bevy_tasks::Scope<'scope, '_, ()>,
        source: &'scope AssetSource,
        path: PathBuf,
    ) -> Result<(), AssetReaderError> {
        if source.reader().is_directory(&path).await? {
            let mut path_stream = source.reader().read_directory(&path).await?;
            while let Some(path) = path_stream.next().await {
                Box::pin(self.process_assets_internal(scope, source, path)).await?;
            }
        } else {
            // Files without extensions are skipped
            let processor = self.clone();
            scope.spawn(async move {
                processor.process_asset(source, path).await;
            });
        }
        Ok(())
    }

    async fn try_reprocessing_queued(&self) {
        loop {
            let mut check_reprocess_queue =
                core::mem::take(&mut self.data.asset_infos.write().await.check_reprocess_queue);
            IoTaskPool::get().scope(|scope| {
                for path in check_reprocess_queue.drain(..) {
                    let processor = self.clone();
                    let source = self.get_source(path.source()).unwrap();
                    scope.spawn(async move {
                        processor.process_asset(source, path.into()).await;
                    });
                }
            });
            let infos = self.data.asset_infos.read().await;
            if infos.check_reprocess_queue.is_empty() {
                break;
            }
        }
    }

    /// Register a new asset processor.
    pub fn register_processor<P: Process>(&self, processor: P) {
        let mut process_plans = self.data.processors.write();
        #[cfg(feature = "trace")]
        let processor = InstrumentedAssetProcessor(processor);
        process_plans.insert(core::any::type_name::<P>(), Arc::new(processor));
    }

    /// Set the default processor for the given `extension`. Make sure `P` is registered with [`AssetProcessor::register_processor`].
    pub fn set_default_processor<P: Process>(&self, extension: &str) {
        let mut default_processors = self.data.default_processors.write();
        default_processors.insert(extension.into(), core::any::type_name::<P>());
    }

    /// Returns the default processor for the given `extension`, if it exists.
    pub fn get_default_processor(&self, extension: &str) -> Option<Arc<dyn ErasedProcessor>> {
        let default_processors = self.data.default_processors.read();
        let key = default_processors.get(extension)?;
        self.data.processors.read().get(key).cloned()
    }

    /// Returns the processor with the given `processor_type_name`, if it exists.
    pub fn get_processor(&self, processor_type_name: &str) -> Option<Arc<dyn ErasedProcessor>> {
        let processors = self.data.processors.read();
        processors.get(processor_type_name).cloned()
    }

    /// Populates the initial view of each asset by scanning the unprocessed and processed asset folders.
    /// This info will later be used to determine whether or not to re-process an asset
    ///
    /// This will validate transactions and recover failed transactions when necessary.
    #[cfg_attr(
        any(target_arch = "wasm32", not(feature = "multi_threaded")),
        expect(
            dead_code,
            reason = "This function is only used when the `multi_threaded` feature is enabled, and when not on WASM."
        )
    )]
    async fn initialize(&self) -> Result<(), InitializeError> {
        self.validate_transaction_log_and_recover().await;
        let mut asset_infos = self.data.asset_infos.write().await;

        /// Retrieves asset paths recursively. If `clean_empty_folders_writer` is Some, it will be used to clean up empty
        /// folders when they are discovered.
        async fn get_asset_paths(
            reader: &dyn ErasedAssetReader,
            clean_empty_folders_writer: Option<&dyn ErasedAssetWriter>,
            path: PathBuf,
            paths: &mut Vec<PathBuf>,
        ) -> Result<bool, AssetReaderError> {
            if reader.is_directory(&path).await? {
                let mut path_stream = reader.read_directory(&path).await?;
                let mut contains_files = false;

                while let Some(child_path) = path_stream.next().await {
                    contains_files |= Box::pin(get_asset_paths(
                        reader,
                        clean_empty_folders_writer,
                        child_path,
                        paths,
                    ))
                    .await?;
                }
                if !contains_files
                    && path.parent().is_some()
                    && let Some(writer) = clean_empty_folders_writer
                {
                    // it is ok for this to fail as it is just a cleanup job.
                    let _ = writer.remove_empty_directory(&path).await;
                }
                Ok(contains_files)
            } else {
                paths.push(path);
                Ok(true)
            }
        }

        for source in self.sources().iter_processed() {
            let Ok(processed_reader) = source.processed_reader() else {
                continue;
            };
            let Ok(processed_writer) = source.processed_writer() else {
                continue;
            };
            let mut unprocessed_paths = Vec::new();
            get_asset_paths(
                source.reader(),
                None,
                PathBuf::from(""),
                &mut unprocessed_paths,
            )
            .await
            .map_err(InitializeError::FailedToReadSourcePaths)?;

            let mut processed_paths = Vec::new();
            get_asset_paths(
                processed_reader,
                Some(processed_writer),
                PathBuf::from(""),
                &mut processed_paths,
            )
            .await
            .map_err(InitializeError::FailedToReadDestinationPaths)?;

            for path in unprocessed_paths {
                asset_infos.get_or_insert(AssetPath::from(path).with_source(source.id()));
            }

            for path in processed_paths {
                let mut dependencies = Vec::new();
                let asset_path = AssetPath::from(path).with_source(source.id());
                if let Some(info) = asset_infos.get_mut(&asset_path) {
                    match processed_reader.read_meta_bytes(asset_path.path()).await {
                        Ok(meta_bytes) => {
                            match ron::de::from_bytes::<ProcessedInfoMinimal>(&meta_bytes) {
                                Ok(minimal) => {
                                    trace!(
                                        "Populated processed info for asset {asset_path} {:?}",
                                        minimal.processed_info
                                    );

                                    if let Some(processed_info) = &minimal.processed_info {
                                        for process_dependency_info in
                                            &processed_info.process_dependencies
                                        {
                                            dependencies.push(process_dependency_info.path.clone());
                                        }
                                    }
                                    info.processed_info = minimal.processed_info;
                                }
                                Err(err) => {
                                    trace!("Removing processed data for {asset_path} because meta could not be parsed: {err}");
                                    self.remove_processed_asset_and_meta(source, asset_path.path())
                                        .await;
                                }
                            }
                        }
                        Err(err) => {
                            trace!("Removing processed data for {asset_path} because meta failed to load: {err}");
                            self.remove_processed_asset_and_meta(source, asset_path.path())
                                .await;
                        }
                    }
                } else {
                    trace!("Removing processed data for non-existent asset {asset_path}");
                    self.remove_processed_asset_and_meta(source, asset_path.path())
                        .await;
                }

                for dependency in dependencies {
                    asset_infos.add_dependent(&dependency, asset_path.clone());
                }
            }
        }

        self.set_state(ProcessorState::Processing).await;

        Ok(())
    }

    /// Removes the processed version of an asset and its metadata, if it exists. This _is not_ transactional like `remove_processed_asset_transactional`, nor
    /// does it remove existing in-memory metadata.
    async fn remove_processed_asset_and_meta(&self, source: &AssetSource, path: &Path) {
        if let Err(err) = source.processed_writer().unwrap().remove(path).await {
            warn!("Failed to remove non-existent asset {path:?}: {err}");
        }

        if let Err(err) = source.processed_writer().unwrap().remove_meta(path).await {
            warn!("Failed to remove non-existent meta {path:?}: {err}");
        }

        self.clean_empty_processed_ancestor_folders(source, path)
            .await;
    }

    async fn clean_empty_processed_ancestor_folders(&self, source: &AssetSource, path: &Path) {
        // As a safety precaution don't delete absolute paths to avoid deleting folders outside of the destination folder
        if path.is_absolute() {
            error!("Attempted to clean up ancestor folders of an absolute path. This is unsafe so the operation was skipped.");
            return;
        }
        while let Some(parent) = path.parent() {
            if parent == Path::new("") {
                break;
            }
            if source
                .processed_writer()
                .unwrap()
                .remove_empty_directory(parent)
                .await
                .is_err()
            {
                // if we fail to delete a folder, stop walking up the tree
                break;
            }
        }
    }

    /// Processes the asset (if it has not already been processed or the asset source has changed).
    /// If the asset has "process dependencies" (relies on the values of other assets), it will asynchronously await until
    /// the dependencies have been processed (See [`ProcessorGatedReader`], which is used in the [`AssetProcessor`]'s [`AssetServer`]
    /// to block reads until the asset is processed).
    ///
    /// [`LoadContext`]: crate::loader::LoadContext
    /// [`ProcessorGatedReader`]: crate::io::processor_gated::ProcessorGatedReader
    async fn process_asset(&self, source: &AssetSource, path: PathBuf) {
        let asset_path = AssetPath::from(path).with_source(source.id());
        let result = self.process_asset_internal(source, &asset_path).await;
        let mut infos = self.data.asset_infos.write().await;
        infos.finish_processing(asset_path, result).await;
    }

    async fn process_asset_internal(
        &self,
        source: &AssetSource,
        asset_path: &AssetPath<'static>,
    ) -> Result<ProcessResult, ProcessError> {
        // TODO: The extension check was removed now that AssetPath is the input. is that ok?
        // TODO: check if already processing to protect against duplicate hot-reload events
        debug!("Processing {}", asset_path);
        let server = &self.server;
        let path = asset_path.path();
        let reader = source.reader();

        let reader_err = |err| ProcessError::AssetReaderError {
            path: asset_path.clone(),
            err,
        };
        let writer_err = |err| ProcessError::AssetWriterError {
            path: asset_path.clone(),
            err,
        };

        // Note: we get the asset source reader first because we don't want to create meta files for assets that don't have source files
        let mut byte_reader = reader.read(path).await.map_err(reader_err)?;

        let (mut source_meta, meta_bytes, processor) = match reader.read_meta_bytes(path).await {
            Ok(meta_bytes) => {
                let minimal: AssetMetaMinimal = ron::de::from_bytes(&meta_bytes).map_err(|e| {
                    ProcessError::DeserializeMetaError(DeserializeMetaError::DeserializeMinimal(e))
                })?;
                let (meta, processor) = match minimal.asset {
                    AssetActionMinimal::Load { loader } => {
                        let loader = server.get_asset_loader_with_type_name(&loader).await?;
                        let meta = loader.deserialize_meta(&meta_bytes)?;
                        (meta, None)
                    }
                    AssetActionMinimal::Process { processor } => {
                        let processor = self
                            .get_processor(&processor)
                            .ok_or_else(|| ProcessError::MissingProcessor(processor))?;
                        let meta = processor.deserialize_meta(&meta_bytes)?;
                        (meta, Some(processor))
                    }
                    AssetActionMinimal::Ignore => {
                        return Ok(ProcessResult::Ignored);
                    }
                };
                (meta, meta_bytes, processor)
            }
            Err(AssetReaderError::NotFound(_path)) => {
                let (meta, processor) = if let Some(processor) = asset_path
                    .get_full_extension()
                    .and_then(|ext| self.get_default_processor(&ext))
                {
                    let meta = processor.default_meta();
                    (meta, Some(processor))
                } else {
                    match server.get_path_asset_loader(asset_path.clone()).await {
                        Ok(loader) => (loader.default_meta(), None),
                        Err(MissingAssetLoaderForExtensionError { .. }) => {
                            let meta: Box<dyn AssetMetaDyn> =
                                Box::new(AssetMeta::<(), ()>::new(AssetAction::Ignore));
                            (meta, None)
                        }
                    }
                };
                let meta_bytes = meta.serialize();
                (meta, meta_bytes, processor)
            }
            Err(err) => {
                return Err(ProcessError::ReadAssetMetaError {
                    path: asset_path.clone(),
                    err,
                })
            }
        };

        let processed_writer = source.processed_writer()?;

        let mut asset_bytes = Vec::new();
        byte_reader
            .read_to_end(&mut asset_bytes)
            .await
            .map_err(|e| ProcessError::AssetReaderError {
                path: asset_path.clone(),
                err: AssetReaderError::Io(e.into()),
            })?;

        // PERF: in theory these hashes could be streamed if we want to avoid allocating the whole asset.
        // The downside is that reading assets would need to happen twice (once for the hash and once for the asset loader)
        // Hard to say which is worse
        let new_hash = get_asset_hash(&meta_bytes, &asset_bytes);
        let mut new_processed_info = ProcessedInfo {
            hash: new_hash,
            full_hash: new_hash,
            process_dependencies: Vec::new(),
        };

        {
            let infos = self.data.asset_infos.read().await;
            if let Some(current_processed_info) = infos
                .get(asset_path)
                .and_then(|i| i.processed_info.as_ref())
                && current_processed_info.hash == new_hash
            {
                let mut dependency_changed = false;
                for current_dep_info in &current_processed_info.process_dependencies {
                    let live_hash = infos
                        .get(&current_dep_info.path)
                        .and_then(|i| i.processed_info.as_ref())
                        .map(|i| i.full_hash);
                    if live_hash != Some(current_dep_info.full_hash) {
                        dependency_changed = true;
                        break;
                    }
                }
                if !dependency_changed {
                    return Ok(ProcessResult::SkippedNotChanged);
                }
            }
        }
        // Note: this lock must remain alive until all processed asset and meta writes have finished (or failed)
        // See ProcessedAssetInfo::file_transaction_lock docs for more info
        let _transaction_lock = {
            let mut infos = self.data.asset_infos.write().await;
            let info = infos.get_or_insert(asset_path.clone());
            info.file_transaction_lock.write_arc().await
        };

        // NOTE: if processing the asset fails this will produce an "unfinished" log entry, forcing a rebuild on next run.
        // Directly writing to the asset destination in the processor necessitates this behavior
        // TODO: this class of failure can be recovered via re-processing + smarter log validation that allows for duplicate transactions in the event of failures
        self.log_begin_processing(asset_path).await;
        if let Some(processor) = processor {
            let mut writer = processed_writer.write(path).await.map_err(writer_err)?;
            let mut processed_meta = {
                let mut context =
                    ProcessContext::new(self, asset_path, &asset_bytes, &mut new_processed_info);
                processor
                    .process(&mut context, source_meta, &mut *writer)
                    .await?
            };

            writer
                .flush()
                .await
                .map_err(|e| ProcessError::AssetWriterError {
                    path: asset_path.clone(),
                    err: AssetWriterError::Io(e),
                })?;

            let full_hash = get_full_asset_hash(
                new_hash,
                new_processed_info
                    .process_dependencies
                    .iter()
                    .map(|i| i.full_hash),
            );
            new_processed_info.full_hash = full_hash;
            *processed_meta.processed_info_mut() = Some(new_processed_info.clone());
            let meta_bytes = processed_meta.serialize();
            processed_writer
                .write_meta_bytes(path, &meta_bytes)
                .await
                .map_err(writer_err)?;
        } else {
            processed_writer
                .write_bytes(path, &asset_bytes)
                .await
                .map_err(writer_err)?;
            *source_meta.processed_info_mut() = Some(new_processed_info.clone());
            let meta_bytes = source_meta.serialize();
            processed_writer
                .write_meta_bytes(path, &meta_bytes)
                .await
                .map_err(writer_err)?;
        }
        self.log_end_processing(asset_path).await;

        Ok(ProcessResult::Processed(new_processed_info))
    }

    async fn validate_transaction_log_and_recover(&self) {
        if let Err(err) = ProcessorTransactionLog::validate().await {
            let state_is_valid = match err {
                ValidateLogError::ReadLogError(err) => {
                    error!("Failed to read processor log file. Processed assets cannot be validated so they must be re-generated {err}");
                    false
                }
                ValidateLogError::UnrecoverableError => {
                    error!("Encountered an unrecoverable error in the last run. Processed assets cannot be validated so they must be re-generated");
                    false
                }
                ValidateLogError::EntryErrors(entry_errors) => {
                    let mut state_is_valid = true;
                    for entry_error in entry_errors {
                        match entry_error {
                            LogEntryError::DuplicateTransaction(_)
                            | LogEntryError::EndedMissingTransaction(_) => {
                                error!("{}", entry_error);
                                state_is_valid = false;
                                break;
                            }
                            LogEntryError::UnfinishedTransaction(path) => {
                                debug!("Asset {path:?} did not finish processing. Clearing state for that asset");
                                let mut unrecoverable_err = |message: &dyn core::fmt::Display| {
                                    error!("Failed to remove asset {path:?}: {message}");
                                    state_is_valid = false;
                                };
                                let Ok(source) = self.get_source(path.source()) else {
                                    unrecoverable_err(&"AssetSource does not exist");
                                    continue;
                                };
                                let Ok(processed_writer) = source.processed_writer() else {
                                    unrecoverable_err(&"AssetSource does not have a processed AssetWriter registered");
                                    continue;
                                };

                                if let Err(err) = processed_writer.remove(path.path()).await {
                                    match err {
                                        AssetWriterError::Io(err) => {
                                            // any error but NotFound means we could be in a bad state
                                            if err.kind() != ErrorKind::NotFound {
                                                unrecoverable_err(&err);
                                            }
                                        }
                                    }
                                }
                                if let Err(err) = processed_writer.remove_meta(path.path()).await {
                                    match err {
                                        AssetWriterError::Io(err) => {
                                            // any error but NotFound means we could be in a bad state
                                            if err.kind() != ErrorKind::NotFound {
                                                unrecoverable_err(&err);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    state_is_valid
                }
            };

            if !state_is_valid {
                error!("Processed asset transaction log state was invalid and unrecoverable for some reason (see previous logs). Removing processed assets and starting fresh.");
                for source in self.sources().iter_processed() {
                    let Ok(processed_writer) = source.processed_writer() else {
                        continue;
                    };
                    if let Err(err) = processed_writer
                        .remove_assets_in_directory(Path::new(""))
                        .await
                    {
                        panic!("Processed assets were in a bad state. To correct this, the asset processor attempted to remove all processed assets and start from scratch. This failed. There is no way to continue. Try restarting, or deleting imported asset folder manually. {err}");
                    }
                }
            }
        }
        let mut log = self.data.log.write().await;
        *log = match ProcessorTransactionLog::new().await {
            Ok(log) => Some(log),
            Err(err) => panic!("Failed to initialize asset processor log. This cannot be recovered. Try restarting. If that doesn't work, try deleting processed asset folder. {}", err),
        };
    }
}

impl AssetProcessorData {
    /// Initializes a new [`AssetProcessorData`] using the given [`AssetSources`].
    pub fn new(source: AssetSources) -> Self {
        let (mut finished_sender, finished_receiver) = async_broadcast::broadcast(1);
        let (mut initialized_sender, initialized_receiver) = async_broadcast::broadcast(1);
        // allow overflow on these "one slot" channels to allow receivers to retrieve the "latest" state, and to allow senders to
        // not block if there was older state present.
        finished_sender.set_overflow(true);
        initialized_sender.set_overflow(true);

        AssetProcessorData {
            sources: source,
            finished_sender,
            finished_receiver,
            initialized_sender,
            initialized_receiver,
            state: async_lock::RwLock::new(ProcessorState::Initializing),
            log: Default::default(),
            processors: Default::default(),
            asset_infos: Default::default(),
            default_processors: Default::default(),
        }
    }

    /// Returns a future that will not finish until the path has been processed.
    pub async fn wait_until_processed(&self, path: AssetPath<'static>) -> ProcessStatus {
        self.wait_until_initialized().await;
        let mut receiver = {
            let infos = self.asset_infos.write().await;
            let info = infos.get(&path);
            match info {
                Some(info) => match info.status {
                    Some(result) => return result,
                    // This receiver must be created prior to losing the read lock to ensure this is transactional
                    None => info.status_receiver.clone(),
                },
                None => return ProcessStatus::NonExistent,
            }
        };
        receiver.recv().await.unwrap()
    }

    /// Returns a future that will not finish until the processor has been initialized.
    pub async fn wait_until_initialized(&self) {
        let receiver = {
            let state = self.state.read().await;
            match *state {
                ProcessorState::Initializing => {
                    // This receiver must be created prior to losing the read lock to ensure this is transactional
                    Some(self.initialized_receiver.clone())
                }
                _ => None,
            }
        };

        if let Some(mut receiver) = receiver {
            receiver.recv().await.unwrap();
        }
    }

    /// Returns a future that will not finish until processing has finished.
    pub async fn wait_until_finished(&self) {
        let receiver = {
            let state = self.state.read().await;
            match *state {
                ProcessorState::Initializing | ProcessorState::Processing => {
                    // This receiver must be created prior to losing the read lock to ensure this is transactional
                    Some(self.finished_receiver.clone())
                }
                ProcessorState::Finished => None,
            }
        };

        if let Some(mut receiver) = receiver {
            receiver.recv().await.unwrap();
        }
    }
}

#[cfg(feature = "trace")]
struct InstrumentedAssetProcessor<T>(T);

#[cfg(feature = "trace")]
impl<T: Process> Process for InstrumentedAssetProcessor<T> {
    type Settings = T::Settings;
    type OutputLoader = T::OutputLoader;

    fn process(
        &self,
        context: &mut ProcessContext,
        meta: AssetMeta<(), Self>,
        writer: &mut crate::io::Writer,
    ) -> impl ConditionalSendFuture<
        Output = Result<<Self::OutputLoader as crate::AssetLoader>::Settings, ProcessError>,
    > {
        // Change the processor type for the `AssetMeta`, which works because we share the `Settings` type.
        let meta = AssetMeta {
            meta_format_version: meta.meta_format_version,
            processed_info: meta.processed_info,
            asset: meta.asset,
        };
        let span = info_span!(
            "asset processing",
            processor = core::any::type_name::<T>(),
            asset = context.path().to_string(),
        );
        self.0.process(context, meta, writer).instrument(span)
    }
}

/// The (successful) result of processing an asset
#[derive(Debug, Clone)]
pub enum ProcessResult {
    Processed(ProcessedInfo),
    SkippedNotChanged,
    Ignored,
}

/// The final status of processing an asset
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ProcessStatus {
    Processed,
    Failed,
    NonExistent,
}

// NOTE: if you add new fields to this struct, make sure they are propagated (when relevant) in ProcessorAssetInfos::rename
#[derive(Debug)]
pub(crate) struct ProcessorAssetInfo {
    processed_info: Option<ProcessedInfo>,
    /// Paths of assets that depend on this asset when they are being processed.
    dependents: HashSet<AssetPath<'static>>,
    status: Option<ProcessStatus>,
    /// A lock that controls read/write access to processed asset files. The lock is shared for both the asset bytes and the meta bytes.
    /// _This lock must be locked whenever a read or write to processed assets occurs_
    /// There are scenarios where processed assets (and their metadata) are being read and written in multiple places at once:
    /// * when the processor is running in parallel with an app
    /// * when processing assets in parallel, the processor might read an asset's `process_dependencies` when processing new versions of those dependencies
    ///     * this second scenario almost certainly isn't possible with the current implementation, but its worth protecting against
    ///
    /// This lock defends against those scenarios by ensuring readers don't read while processed files are being written. And it ensures
    /// Because this lock is shared across meta and asset bytes, readers can ensure they don't read "old" versions of metadata with "new" asset data.
    pub(crate) file_transaction_lock: Arc<async_lock::RwLock<()>>,
    status_sender: async_broadcast::Sender<ProcessStatus>,
    status_receiver: async_broadcast::Receiver<ProcessStatus>,
}

impl Default for ProcessorAssetInfo {
    fn default() -> Self {
        let (mut status_sender, status_receiver) = async_broadcast::broadcast(1);
        // allow overflow on these "one slot" channels to allow receivers to retrieve the "latest" state, and to allow senders to
        // not block if there was older state present.
        status_sender.set_overflow(true);
        Self {
            processed_info: Default::default(),
            dependents: Default::default(),
            file_transaction_lock: Default::default(),
            status: None,
            status_sender,
            status_receiver,
        }
    }
}

impl ProcessorAssetInfo {
    async fn update_status(&mut self, status: ProcessStatus) {
        if self.status != Some(status) {
            self.status = Some(status);
            self.status_sender.broadcast(status).await.unwrap();
        }
    }
}

/// The "current" in memory view of the asset space. This is "eventually consistent". It does not directly
/// represent the state of assets in storage, but rather a valid historical view that will gradually become more
/// consistent as events are processed.
#[derive(Default, Debug)]
pub struct ProcessorAssetInfos {
    /// The "current" in memory view of the asset space. During processing, if path does not exist in this, it should
    /// be considered non-existent.
    /// NOTE: YOU MUST USE `Self::get_or_insert` or `Self::insert` TO ADD ITEMS TO THIS COLLECTION TO ENSURE
    /// `non_existent_dependents` DATA IS CONSUMED
    infos: HashMap<AssetPath<'static>, ProcessorAssetInfo>,
    /// Dependents for assets that don't exist. This exists to track "dangling" asset references due to deleted / missing files.
    /// If the dependent asset is added, it can "resolve" these dependencies and re-compute those assets.
    /// Therefore this _must_ always be consistent with the `infos` data. If a new asset is added to `infos`, it should
    /// check this maps for dependencies and add them. If an asset is removed, it should update the dependents here.
    non_existent_dependents: HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
    check_reprocess_queue: VecDeque<AssetPath<'static>>,
}

impl ProcessorAssetInfos {
    fn get_or_insert(&mut self, asset_path: AssetPath<'static>) -> &mut ProcessorAssetInfo {
        self.infos.entry(asset_path.clone()).or_insert_with(|| {
            let mut info = ProcessorAssetInfo::default();
            // track existing dependents by resolving existing "hanging" dependents.
            if let Some(dependents) = self.non_existent_dependents.remove(&asset_path) {
                info.dependents = dependents;
            }
            info
        })
    }

    pub(crate) fn get(&self, asset_path: &AssetPath<'static>) -> Option<&ProcessorAssetInfo> {
        self.infos.get(asset_path)
    }

    fn get_mut(&mut self, asset_path: &AssetPath<'static>) -> Option<&mut ProcessorAssetInfo> {
        self.infos.get_mut(asset_path)
    }

    fn add_dependent(&mut self, asset_path: &AssetPath<'static>, dependent: AssetPath<'static>) {
        if let Some(info) = self.get_mut(asset_path) {
            info.dependents.insert(dependent);
        } else {
            let dependents = self
                .non_existent_dependents
                .entry(asset_path.clone())
                .or_default();
            dependents.insert(dependent);
        }
    }

    /// Finalize processing the asset, which will incorporate the result of the processed asset into the in-memory view the processed assets.
    async fn finish_processing(
        &mut self,
        asset_path: AssetPath<'static>,
        result: Result<ProcessResult, ProcessError>,
    ) {
        match result {
            Ok(ProcessResult::Processed(processed_info)) => {
                debug!("Finished processing \"{}\"", asset_path);
                // clean up old dependents
                let old_processed_info = self
                    .infos
                    .get_mut(&asset_path)
                    .and_then(|i| i.processed_info.take());
                if let Some(old_processed_info) = old_processed_info {
                    self.clear_dependencies(&asset_path, old_processed_info);
                }

                // populate new dependents
                for process_dependency_info in &processed_info.process_dependencies {
                    self.add_dependent(&process_dependency_info.path, asset_path.to_owned());
                }
                let info = self.get_or_insert(asset_path);
                info.processed_info = Some(processed_info);
                info.update_status(ProcessStatus::Processed).await;
                let dependents = info.dependents.iter().cloned().collect::<Vec<_>>();
                for path in dependents {
                    self.check_reprocess_queue.push_back(path);
                }
            }
            Ok(ProcessResult::SkippedNotChanged) => {
                debug!("Skipping processing (unchanged) \"{}\"", asset_path);
                let info = self.get_mut(&asset_path).expect("info should exist");
                // NOTE: skipping an asset on a given pass doesn't mean it won't change in the future as a result
                // of a dependency being re-processed. This means apps might receive an "old" (but valid) asset first.
                // This is in the interest of fast startup times that don't block for all assets being checked + reprocessed
                // Therefore this relies on hot-reloading in the app to pickup the "latest" version of the asset
                // If "block until latest state is reflected" is required, we can easily add a less granular
                // "block until first pass finished" mode
                info.update_status(ProcessStatus::Processed).await;
            }
            Ok(ProcessResult::Ignored) => {
                debug!("Skipping processing (ignored) \"{}\"", asset_path);
            }
            Err(ProcessError::ExtensionRequired) => {
                // Skip assets without extensions
            }
            Err(ProcessError::MissingAssetLoaderForExtension(_)) => {
                trace!("No loader found for {asset_path}");
            }
            Err(ProcessError::AssetReaderError {
                err: AssetReaderError::NotFound(_),
                ..
            }) => {
                // if there is no asset source, no processing can be done
                trace!("No need to process asset {asset_path} because it does not exist");
            }
            Err(err) => {
                error!("Failed to process asset {asset_path}: {err}");
                // if this failed because a dependency could not be loaded, make sure it is reprocessed if that dependency is reprocessed
                if let ProcessError::AssetLoadError(AssetLoadError::AssetLoaderError(dependency)) =
                    err
                {
                    let info = self.get_mut(&asset_path).expect("info should exist");
                    info.processed_info = Some(ProcessedInfo {
                        hash: AssetHash::default(),
                        full_hash: AssetHash::default(),
                        process_dependencies: vec![],
                    });
                    self.add_dependent(dependency.path(), asset_path.to_owned());
                }

                let info = self.get_mut(&asset_path).expect("info should exist");
                info.update_status(ProcessStatus::Failed).await;
            }
        }
    }

    /// Remove the info for the given path. This should only happen if an asset's source is removed / non-existent
    async fn remove(&mut self, asset_path: &AssetPath<'static>) {
        let info = self.infos.remove(asset_path);
        if let Some(info) = info {
            if let Some(processed_info) = info.processed_info {
                self.clear_dependencies(asset_path, processed_info);
            }
            // Tell all listeners this asset does not exist
            info.status_sender
                .broadcast(ProcessStatus::NonExistent)
                .await
                .unwrap();
            if !info.dependents.is_empty() {
                error!(
                    "The asset at {asset_path} was removed, but it had assets that depend on it to be processed. Consider updating the path in the following assets: {:?}",
                    info.dependents
                );
                self.non_existent_dependents
                    .insert(asset_path.clone(), info.dependents);
            }
        }
    }

    /// Remove the info for the given path. This should only happen if an asset's source is removed / non-existent
    async fn rename(&mut self, old: &AssetPath<'static>, new: &AssetPath<'static>) {
        let info = self.infos.remove(old);
        if let Some(mut info) = info {
            if !info.dependents.is_empty() {
                // TODO: We can't currently ensure "moved" folders with relative paths aren't broken because AssetPath
                // doesn't distinguish between absolute and relative paths. We have "erased" relativeness. In the short term,
                // we could do "remove everything in a folder and re-add", but that requires full rebuilds / destroying the cache.
                // If processors / loaders could enumerate dependencies, we could check if the new deps line up with a rename.
                // If deps encoded "relativeness" as part of loading, that would also work (this seems like the right call).
                // TODO: it would be nice to log an error here for dependents that aren't also being moved + fixed.
                // (see the remove impl).
                error!(
                    "The asset at {old} was removed, but it had assets that depend on it to be processed. Consider updating the path in the following assets: {:?}",
                    info.dependents
                );
                self.non_existent_dependents
                    .insert(old.clone(), core::mem::take(&mut info.dependents));
            }
            if let Some(processed_info) = &info.processed_info {
                // Update "dependent" lists for this asset's "process dependencies" to use new path.
                for dep in &processed_info.process_dependencies {
                    if let Some(info) = self.infos.get_mut(&dep.path) {
                        info.dependents.remove(old);
                        info.dependents.insert(new.clone());
                    } else if let Some(dependents) = self.non_existent_dependents.get_mut(&dep.path)
                    {
                        dependents.remove(old);
                        dependents.insert(new.clone());
                    }
                }
            }
            // Tell all listeners this asset no longer exists
            info.status_sender
                .broadcast(ProcessStatus::NonExistent)
                .await
                .unwrap();
            let dependents: Vec<AssetPath<'static>> = {
                let new_info = self.get_or_insert(new.clone());
                new_info.processed_info = info.processed_info;
                new_info.status = info.status;
                // Ensure things waiting on the new path are informed of the status of this asset
                if let Some(status) = new_info.status {
                    new_info.status_sender.broadcast(status).await.unwrap();
                }
                new_info.dependents.iter().cloned().collect()
            };
            // Queue the asset for a reprocess check, in case it needs new meta.
            self.check_reprocess_queue.push_back(new.clone());
            for dependent in dependents {
                // Queue dependents for reprocessing because they might have been waiting for this asset.
                self.check_reprocess_queue.push_back(dependent);
            }
        }
    }

    fn clear_dependencies(&mut self, asset_path: &AssetPath<'static>, removed_info: ProcessedInfo) {
        for old_load_dep in removed_info.process_dependencies {
            if let Some(info) = self.infos.get_mut(&old_load_dep.path) {
                info.dependents.remove(asset_path);
            } else if let Some(dependents) =
                self.non_existent_dependents.get_mut(&old_load_dep.path)
            {
                dependents.remove(asset_path);
            }
        }
    }
}

/// The current state of the [`AssetProcessor`].
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ProcessorState {
    /// The processor is still initializing, which involves scanning the current asset folders,
    /// constructing an in-memory view of the asset space, recovering from previous errors / crashes,
    /// and cleaning up old / unused assets.
    Initializing,
    /// The processor is currently processing assets.
    Processing,
    /// The processor has finished processing all valid assets and reporting invalid assets.
    Finished,
}

/// An error that occurs when initializing the [`AssetProcessor`].
#[derive(Error, Debug)]
pub enum InitializeError {
    #[error(transparent)]
    FailedToReadSourcePaths(AssetReaderError),
    #[error(transparent)]
    FailedToReadDestinationPaths(AssetReaderError),
    #[error("Failed to validate asset log: {0}")]
    ValidateLogError(#[from] ValidateLogError),
}
