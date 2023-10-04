mod log;
mod process;

pub use log::*;
pub use process::*;

use crate::{
    io::{
        processor_gated::ProcessorGatedReader, AssetProvider, AssetProviders, AssetReader,
        AssetReaderError, AssetSourceEvent, AssetWatcher, AssetWriter, AssetWriterError,
    },
    meta::{
        get_asset_hash, get_full_asset_hash, AssetAction, AssetActionMinimal, AssetHash, AssetMeta,
        AssetMetaDyn, AssetMetaMinimal, ProcessedInfo, ProcessedInfoMinimal,
    },
    AssetLoadError, AssetLoaderError, AssetPath, AssetServer, DeserializeMetaError,
    LoadDirectError, MissingAssetLoaderForExtensionError, CANNOT_WATCH_ERROR_MESSAGE,
};
use bevy_ecs::prelude::*;
use bevy_log::{debug, error, trace, warn};
use bevy_tasks::IoTaskPool;
use bevy_utils::{BoxedFuture, HashMap, HashSet};
use futures_io::ErrorKind;
use futures_lite::{AsyncReadExt, AsyncWriteExt, StreamExt};
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

/// A "background" asset processor that reads asset values from a source [`AssetProvider`] (which corresponds to an [`AssetReader`] / [`AssetWriter`] pair),
/// processes them in some way, and writes them to a destination [`AssetProvider`].
///
/// This will create .meta files (a human-editable serialized form of [`AssetMeta`]) in the source [`AssetProvider`] for assets that
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

pub struct AssetProcessorData {
    pub(crate) asset_infos: async_lock::RwLock<ProcessorAssetInfos>,
    log: async_lock::RwLock<Option<ProcessorTransactionLog>>,
    processors: RwLock<HashMap<&'static str, Arc<dyn ErasedProcessor>>>,
    /// Default processors for file extensions
    default_processors: RwLock<HashMap<String, &'static str>>,
    state: async_lock::RwLock<ProcessorState>,
    source_reader: Box<dyn AssetReader>,
    source_writer: Box<dyn AssetWriter>,
    destination_reader: Box<dyn AssetReader>,
    destination_writer: Box<dyn AssetWriter>,
    initialized_sender: async_broadcast::Sender<()>,
    initialized_receiver: async_broadcast::Receiver<()>,
    finished_sender: async_broadcast::Sender<()>,
    finished_receiver: async_broadcast::Receiver<()>,
    source_event_receiver: crossbeam_channel::Receiver<AssetSourceEvent>,
    _source_watcher: Option<Box<dyn AssetWatcher>>,
}

impl AssetProcessor {
    /// Creates a new [`AssetProcessor`] instance.
    pub fn new(
        providers: &mut AssetProviders,
        source: &AssetProvider,
        destination: &AssetProvider,
    ) -> Self {
        let data = Arc::new(AssetProcessorData::new(
            providers.get_source_reader(source),
            providers.get_source_writer(source),
            providers.get_destination_reader(destination),
            providers.get_destination_writer(destination),
        ));
        let destination_reader = providers.get_destination_reader(destination);
        // The asset processor uses its own asset server with its own id space
        let server = AssetServer::new(
            Box::new(ProcessorGatedReader::new(destination_reader, data.clone())),
            true,
        );
        Self { server, data }
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

    /// Retrieves the "source" [`AssetReader`] (the place where user-provided unprocessed "asset sources" are stored)
    pub fn source_reader(&self) -> &dyn AssetReader {
        &*self.data.source_reader
    }

    /// Retrieves the "source" [`AssetWriter`] (the place where user-provided unprocessed "asset sources" are stored)
    pub fn source_writer(&self) -> &dyn AssetWriter {
        &*self.data.source_writer
    }

    /// Retrieves the "destination" [`AssetReader`] (the place where processed / [`AssetProcessor`]-managed assets are stored)
    pub fn destination_reader(&self) -> &dyn AssetReader {
        &*self.data.destination_reader
    }

    /// Retrieves the "destination" [`AssetWriter`] (the place where processed / [`AssetProcessor`]-managed assets are stored)
    pub fn destination_writer(&self) -> &dyn AssetWriter {
        &*self.data.destination_writer
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
    async fn log_begin_processing(&self, path: &Path) {
        let mut log = self.data.log.write().await;
        let log = log.as_mut().unwrap();
        log.begin_processing(path).await.unwrap();
    }

    /// Logs the end of an asset being successfully processed. See [`AssetProcessor::log_begin_processing`].
    async fn log_end_processing(&self, path: &Path) {
        let mut log = self.data.log.write().await;
        let log = log.as_mut().unwrap();
        log.end_processing(path).await.unwrap();
    }

    /// Starts the processor in a background thread.
    pub fn start(_processor: Res<Self>) {
        #[cfg(any(target_arch = "wasm32", not(feature = "multi-threaded")))]
        error!("Cannot run AssetProcessor in single threaded mode (or WASM) yet.");
        #[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
        {
            let processor = _processor.clone();
            std::thread::spawn(move || {
                processor.process_assets();
                bevy_tasks::block_on(processor.listen_for_source_change_events());
            });
        }
    }

    /// Processes all assets. This will:
    /// * Scan the [`ProcessorTransactionLog`] and recover from any failures detected
    /// * Scan the destination [`AssetProvider`] to build the current view of already processed assets.
    /// * Scan the source [`AssetProvider`] and remove any processed "destination" assets that are invalid or no longer exist.
    /// * For each asset in the `source` [`AssetProvider`], kick off a new "process job", which will process the asset
    /// (if the latest version of the asset has not been processed).
    #[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
    pub fn process_assets(&self) {
        let start_time = std::time::Instant::now();
        debug!("Processing Assets");
        IoTaskPool::get().scope(|scope| {
            scope.spawn(async move {
                self.initialize().await.unwrap();
                let path = PathBuf::from("");
                self.process_assets_internal(scope, path).await.unwrap();
            });
        });
        // This must happen _after_ the scope resolves or it will happen "too early"
        // Don't move this into the async scope above! process_assets is a blocking/sync function this is fine
        bevy_tasks::block_on(self.finish_processing_assets());
        let end_time = std::time::Instant::now();
        debug!("Processing finished in {:?}", end_time - start_time);
    }

    /// Listens for changes to assets in the source [`AssetProvider`] and update state accordingly.
    // PERF: parallelize change event processing
    pub async fn listen_for_source_change_events(&self) {
        debug!("Listening for changes to source assets");
        loop {
            let mut started_processing = false;

            for event in self.data.source_event_receiver.try_iter() {
                if !started_processing {
                    self.set_state(ProcessorState::Processing).await;
                    started_processing = true;
                }

                self.handle_asset_source_event(event).await;
            }

            if started_processing {
                self.finish_processing_assets().await;
            }
        }
    }

    async fn handle_asset_source_event(&self, event: AssetSourceEvent) {
        trace!("{event:?}");
        match event {
            AssetSourceEvent::AddedAsset(path)
            | AssetSourceEvent::AddedMeta(path)
            | AssetSourceEvent::ModifiedAsset(path)
            | AssetSourceEvent::ModifiedMeta(path) => {
                self.process_asset(&path).await;
            }
            AssetSourceEvent::RemovedAsset(path) => {
                self.handle_removed_asset(path).await;
            }
            AssetSourceEvent::RemovedMeta(path) => {
                self.handle_removed_meta(&path).await;
            }
            AssetSourceEvent::AddedFolder(path) => {
                self.handle_added_folder(path).await;
            }
            // NOTE: As a heads up for future devs: this event shouldn't be run in parallel with other events that might
            // touch this folder (ex: the folder might be re-created with new assets). Clean up the old state first.
            // Currently this event handler is not parallel, but it could be (and likely should be) in the future.
            AssetSourceEvent::RemovedFolder(path) => {
                self.handle_removed_folder(&path).await;
            }
            AssetSourceEvent::RenamedAsset { old, new } => {
                // If there was a rename event, but the path hasn't changed, this asset might need reprocessing.
                // Sometimes this event is returned when an asset is moved "back" into the asset folder
                if old == new {
                    self.process_asset(&new).await;
                } else {
                    self.handle_renamed_asset(old, new).await;
                }
            }
            AssetSourceEvent::RenamedMeta { old, new } => {
                // If there was a rename event, but the path hasn't changed, this asset meta might need reprocessing.
                // Sometimes this event is returned when an asset meta is moved "back" into the asset folder
                if old == new {
                    self.process_asset(&new).await;
                } else {
                    debug!("Meta renamed from {old:?} to {new:?}");
                    let mut infos = self.data.asset_infos.write().await;
                    // Renaming meta should not assume that an asset has also been renamed. Check both old and new assets to see
                    // if they should be re-imported (and/or have new meta generated)
                    infos.check_reprocess_queue.push_back(old);
                    infos.check_reprocess_queue.push_back(new);
                }
            }
            AssetSourceEvent::RenamedFolder { old, new } => {
                // If there was a rename event, but the path hasn't changed, this asset folder might need reprocessing.
                // Sometimes this event is returned when an asset meta is moved "back" into the asset folder
                if old == new {
                    self.handle_added_folder(new).await;
                } else {
                    // PERF: this reprocesses everything in the moved folder. this is not necessary in most cases, but
                    // requires some nuance when it comes to path handling.
                    self.handle_removed_folder(&old).await;
                    self.handle_added_folder(new).await;
                }
            }
            AssetSourceEvent::RemovedUnknown { path, is_meta } => {
                match self.destination_reader().is_directory(&path).await {
                    Ok(is_directory) => {
                        if is_directory {
                            self.handle_removed_folder(&path).await;
                        } else if is_meta {
                            self.handle_removed_meta(&path).await;
                        } else {
                            self.handle_removed_asset(path).await;
                        }
                    }
                    Err(err) => {
                        if let AssetReaderError::NotFound(_) = err {
                            // if the path is not found, a processed version does not exist
                        } else {
                            error!(
                                "Path '{path:?}' as removed, but the destination reader could not determine if it \
                                was a folder or a file due to the following error: {err}"
                            );
                        }
                    }
                }
            }
        }
    }

    async fn handle_added_folder(&self, path: PathBuf) {
        debug!("Folder {:?} was added. Attempting to re-process", path);
        #[cfg(any(target_arch = "wasm32", not(feature = "multi-threaded")))]
        error!("AddFolder event cannot be handled in single threaded mode (or WASM) yet.");
        #[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
        IoTaskPool::get().scope(|scope| {
            scope.spawn(async move {
                self.process_assets_internal(scope, path).await.unwrap();
            });
        });
    }

    /// Responds to a removed meta event by reprocessing the asset at the given path.
    async fn handle_removed_meta(&self, path: &Path) {
        // If meta was removed, we might need to regenerate it.
        // Likewise, the user might be manually re-adding the asset.
        // Therefore, we shouldn't automatically delete the asset ... that is a
        // user-initiated action.
        debug!(
            "Meta for asset {:?} was removed. Attempting to re-process",
            path
        );
        self.process_asset(path).await;
    }

    /// Removes all processed assets stored at the given path (respecting transactionality), then removes the folder itself.
    async fn handle_removed_folder(&self, path: &Path) {
        debug!("Removing folder {:?} because source was removed", path);
        match self.destination_reader().read_directory(path).await {
            Ok(mut path_stream) => {
                while let Some(child_path) = path_stream.next().await {
                    self.handle_removed_asset(child_path).await;
                }
            }
            Err(err) => match err {
                AssetReaderError::NotFound(_err) => {
                    // The processed folder does not exist. No need to update anything
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
        if let Err(AssetWriterError::Io(err)) =
            self.destination_writer().remove_directory(path).await
        {
            // we can ignore NotFound because if the "final" file in a folder was removed
            // then we automatically clean up this folder
            if err.kind() != ErrorKind::NotFound {
                error!("Failed to remove destination folder that no longer exists in asset source {path:?}: {err}");
            }
        }
    }

    /// Removes the processed version of an asset and associated in-memory metadata. This will block until all existing reads/writes to the
    /// asset have finished, thanks to the `file_transaction_lock`.
    async fn handle_removed_asset(&self, path: PathBuf) {
        debug!("Removing processed {:?} because source was removed", path);
        let asset_path = AssetPath::from_path(path);
        let mut infos = self.data.asset_infos.write().await;
        if let Some(info) = infos.get(&asset_path) {
            // we must wait for uncontested write access to the asset source to ensure existing readers / writers
            // can finish their operations
            let _write_lock = info.file_transaction_lock.write();
            self.remove_processed_asset_and_meta(asset_path.path())
                .await;
        }
        infos.remove(&asset_path).await;
    }

    /// Handles a renamed source asset by moving it's processed results to the new location and updating in-memory paths + metadata.
    /// This will cause direct path dependencies to break.
    async fn handle_renamed_asset(&self, old: PathBuf, new: PathBuf) {
        let mut infos = self.data.asset_infos.write().await;
        let old_asset_path = AssetPath::from_path(old);
        if let Some(info) = infos.get(&old_asset_path) {
            // we must wait for uncontested write access to the asset source to ensure existing readers / writers
            // can finish their operations
            let _write_lock = info.file_transaction_lock.write();
            let old = old_asset_path.path();
            self.destination_writer().rename(old, &new).await.unwrap();
            self.destination_writer()
                .rename_meta(old, &new)
                .await
                .unwrap();
        }
        let new_asset_path = AssetPath::from_path(new);
        infos.rename(&old_asset_path, &new_asset_path).await;
    }

    async fn finish_processing_assets(&self) {
        self.try_reprocessing_queued().await;
        // clean up metadata in asset server
        self.server.data.infos.write().consume_handle_drop_events();
        self.set_state(ProcessorState::Finished).await;
    }

    #[allow(unused)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
    fn process_assets_internal<'scope>(
        &'scope self,
        scope: &'scope bevy_tasks::Scope<'scope, '_, ()>,
        path: PathBuf,
    ) -> bevy_utils::BoxedFuture<'scope, Result<(), AssetReaderError>> {
        Box::pin(async move {
            if self.source_reader().is_directory(&path).await? {
                let mut path_stream = self.source_reader().read_directory(&path).await?;
                while let Some(path) = path_stream.next().await {
                    self.process_assets_internal(scope, path).await?;
                }
            } else {
                // Files without extensions are skipped
                let processor = self.clone();
                scope.spawn(async move {
                    processor.process_asset(&path).await;
                });
            }
            Ok(())
        })
    }

    async fn try_reprocessing_queued(&self) {
        loop {
            let mut check_reprocess_queue =
                std::mem::take(&mut self.data.asset_infos.write().await.check_reprocess_queue);
            IoTaskPool::get().scope(|scope| {
                for path in check_reprocess_queue.drain(..) {
                    let processor = self.clone();
                    scope.spawn(async move {
                        processor.process_asset(&path).await;
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
        process_plans.insert(std::any::type_name::<P>(), Arc::new(processor));
    }

    /// Set the default processor for the given `extension`. Make sure `P` is registered with [`AssetProcessor::register_processor`].
    pub fn set_default_processor<P: Process>(&self, extension: &str) {
        let mut default_processors = self.data.default_processors.write();
        default_processors.insert(extension.to_string(), std::any::type_name::<P>());
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

    /// Populates the initial view of each asset by scanning the source and destination folders.
    /// This info will later be used to determine whether or not to re-process an asset
    ///
    /// This will validate transactions and recover failed transactions when necessary.
    #[allow(unused)]
    async fn initialize(&self) -> Result<(), InitializeError> {
        self.validate_transaction_log_and_recover().await;
        let mut asset_infos = self.data.asset_infos.write().await;

        /// Retrieves asset paths recursively. If `clean_empty_folders_writer` is Some, it will be used to clean up empty
        /// folders when they are discovered.
        fn get_asset_paths<'a>(
            reader: &'a dyn AssetReader,
            clean_empty_folders_writer: Option<&'a dyn AssetWriter>,
            path: PathBuf,
            paths: &'a mut Vec<PathBuf>,
        ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
            Box::pin(async move {
                if reader.is_directory(&path).await? {
                    let mut path_stream = reader.read_directory(&path).await?;
                    let mut contains_files = false;
                    while let Some(child_path) = path_stream.next().await {
                        contains_files =
                            get_asset_paths(reader, clean_empty_folders_writer, child_path, paths)
                                .await?
                                && contains_files;
                    }
                    if !contains_files {
                        if let Some(writer) = clean_empty_folders_writer {
                            // it is ok for this to fail as it is just a cleanup job.
                            let _ = writer.remove_empty_directory(&path).await;
                        }
                    }
                    Ok(contains_files)
                } else {
                    paths.push(path);
                    Ok(true)
                }
            })
        }

        let mut source_paths = Vec::new();
        let source_reader = self.source_reader();
        get_asset_paths(source_reader, None, PathBuf::from(""), &mut source_paths)
            .await
            .map_err(InitializeError::FailedToReadSourcePaths)?;

        let mut destination_paths = Vec::new();
        let destination_reader = self.destination_reader();
        let destination_writer = self.destination_writer();
        get_asset_paths(
            destination_reader,
            Some(destination_writer),
            PathBuf::from(""),
            &mut destination_paths,
        )
        .await
        .map_err(InitializeError::FailedToReadDestinationPaths)?;

        for path in &source_paths {
            asset_infos.get_or_insert(AssetPath::from_path(path.clone()));
        }

        for path in &destination_paths {
            let asset_path = AssetPath::from_path(path.clone());
            let mut dependencies = Vec::new();
            if let Some(info) = asset_infos.get_mut(&asset_path) {
                match self.destination_reader().read_meta_bytes(path).await {
                    Ok(meta_bytes) => {
                        match ron::de::from_bytes::<ProcessedInfoMinimal>(&meta_bytes) {
                            Ok(minimal) => {
                                trace!(
                                    "Populated processed info for asset {path:?} {:?}",
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
                                trace!("Removing processed data for {path:?} because meta could not be parsed: {err}");
                                self.remove_processed_asset_and_meta(path).await;
                            }
                        }
                    }
                    Err(err) => {
                        trace!("Removing processed data for {path:?} because meta failed to load: {err}");
                        self.remove_processed_asset_and_meta(path).await;
                    }
                }
            } else {
                trace!("Removing processed data for non-existent asset {path:?}");
                self.remove_processed_asset_and_meta(path).await;
            }

            for dependency in dependencies {
                asset_infos.add_dependant(&dependency, asset_path.clone());
            }
        }

        self.set_state(ProcessorState::Processing).await;

        Ok(())
    }

    /// Removes the processed version of an asset and its metadata, if it exists. This _is not_ transactional like `remove_processed_asset_transactional`, nor
    /// does it remove existing in-memory metadata.
    async fn remove_processed_asset_and_meta(&self, path: &Path) {
        if let Err(err) = self.destination_writer().remove(path).await {
            warn!("Failed to remove non-existent asset {path:?}: {err}");
        }

        if let Err(err) = self.destination_writer().remove_meta(path).await {
            warn!("Failed to remove non-existent meta {path:?}: {err}");
        }

        self.clean_empty_processed_ancestor_folders(path).await;
    }

    async fn clean_empty_processed_ancestor_folders(&self, path: &Path) {
        // As a safety precaution don't delete absolute paths to avoid deleting folders outside of the destination folder
        if path.is_absolute() {
            error!("Attempted to clean up ancestor folders of an absolute path. This is unsafe so the operation was skipped.");
            return;
        }
        while let Some(parent) = path.parent() {
            if parent == Path::new("") {
                break;
            }
            if self
                .destination_writer()
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
    async fn process_asset(&self, path: &Path) {
        let result = self.process_asset_internal(path).await;
        let mut infos = self.data.asset_infos.write().await;
        let asset_path = AssetPath::from_path(path.to_owned());
        infos.finish_processing(asset_path, result).await;
    }

    async fn process_asset_internal(&self, path: &Path) -> Result<ProcessResult, ProcessError> {
        if path.extension().is_none() {
            return Err(ProcessError::ExtensionRequired);
        }
        let asset_path = AssetPath::from_path(path.to_path_buf());
        // TODO: check if already processing to protect against duplicate hot-reload events
        debug!("Processing {:?}", path);
        let server = &self.server;

        // Note: we get the asset source reader first because we don't want to create meta files for assets that don't have source files
        let mut reader = self.source_reader().read(path).await.map_err(|e| match e {
            AssetReaderError::NotFound(_) => ProcessError::MissingAssetSource(path.to_owned()),
            AssetReaderError::Io(err) => ProcessError::AssetSourceIoError(err),
        })?;

        let (mut source_meta, meta_bytes, processor) = match self
            .source_reader()
            .read_meta_bytes(path)
            .await
        {
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
                        let meta: Box<dyn AssetMetaDyn> =
                            Box::new(AssetMeta::<(), ()>::deserialize(&meta_bytes)?);
                        (meta, None)
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
                    match server.get_path_asset_loader(&asset_path).await {
                        Ok(loader) => (loader.default_meta(), None),
                        Err(MissingAssetLoaderForExtensionError { .. }) => {
                            let meta: Box<dyn AssetMetaDyn> =
                                Box::new(AssetMeta::<(), ()>::new(AssetAction::Ignore));
                            (meta, None)
                        }
                    }
                };
                let meta_bytes = meta.serialize();
                // write meta to source location if it doesn't already exist
                self.source_writer()
                    .write_meta_bytes(path, &meta_bytes)
                    .await?;
                (meta, meta_bytes, processor)
            }
            Err(err) => return Err(ProcessError::ReadAssetMetaError(err)),
        };

        let mut asset_bytes = Vec::new();
        reader
            .read_to_end(&mut asset_bytes)
            .await
            .map_err(ProcessError::AssetSourceIoError)?;

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
                .get(&asset_path)
                .and_then(|i| i.processed_info.as_ref())
            {
                if current_processed_info.hash == new_hash {
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
        }
        // Note: this lock must remain alive until all processed asset asset and meta writes have finished (or failed)
        // See ProcessedAssetInfo::file_transaction_lock docs for more info
        let _transaction_lock = {
            let mut infos = self.data.asset_infos.write().await;
            let info = infos.get_or_insert(asset_path.clone());
            info.file_transaction_lock.write_arc().await
        };

        // NOTE: if processing the asset fails this will produce an "unfinished" log entry, forcing a rebuild on next run.
        // Directly writing to the asset destination in the processor necessitates this behavior
        // TODO: this class of failure can be recovered via re-processing + smarter log validation that allows for duplicate transactions in the event of failures
        self.log_begin_processing(path).await;
        if let Some(processor) = processor {
            let mut writer = self.destination_writer().write(path).await?;
            let mut processed_meta = {
                let mut context =
                    ProcessContext::new(self, &asset_path, &asset_bytes, &mut new_processed_info);
                processor
                    .process(&mut context, source_meta, &mut *writer)
                    .await?
            };

            writer.flush().await.map_err(AssetWriterError::Io)?;

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
            self.destination_writer()
                .write_meta_bytes(path, &meta_bytes)
                .await?;
        } else {
            self.destination_writer()
                .write_bytes(path, &asset_bytes)
                .await?;
            *source_meta.processed_info_mut() = Some(new_processed_info.clone());
            let meta_bytes = source_meta.serialize();
            self.destination_writer()
                .write_meta_bytes(path, &meta_bytes)
                .await?;
        }
        self.log_end_processing(path).await;

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
                                if let Err(err) = self.destination_writer().remove(&path).await {
                                    match err {
                                        AssetWriterError::Io(err) => {
                                            // any error but NotFound means we could be in a bad state
                                            if err.kind() != ErrorKind::NotFound {
                                                error!("Failed to remove asset {path:?}: {err}");
                                                state_is_valid = false;
                                            }
                                        }
                                    }
                                }
                                if let Err(err) = self.destination_writer().remove_meta(&path).await
                                {
                                    match err {
                                        AssetWriterError::Io(err) => {
                                            // any error but NotFound means we could be in a bad state
                                            if err.kind() != ErrorKind::NotFound {
                                                error!(
                                                    "Failed to remove asset meta {path:?}: {err}"
                                                );
                                                state_is_valid = false;
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
                if let Err(err) = self
                    .destination_writer()
                    .remove_assets_in_directory(Path::new(""))
                    .await
                {
                    panic!("Processed assets were in a bad state. To correct this, the asset processor attempted to remove all processed assets and start from scratch. This failed. There is no way to continue. Try restarting, or deleting imported asset folder manually. {err}");
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
    pub fn new(
        source_reader: Box<dyn AssetReader>,
        source_writer: Box<dyn AssetWriter>,
        destination_reader: Box<dyn AssetReader>,
        destination_writer: Box<dyn AssetWriter>,
    ) -> Self {
        let (mut finished_sender, finished_receiver) = async_broadcast::broadcast(1);
        let (mut initialized_sender, initialized_receiver) = async_broadcast::broadcast(1);
        // allow overflow on these "one slot" channels to allow receivers to retrieve the "latest" state, and to allow senders to
        // not block if there was older state present.
        finished_sender.set_overflow(true);
        initialized_sender.set_overflow(true);
        let (source_event_sender, source_event_receiver) = crossbeam_channel::unbounded();
        // TODO: watching for changes could probably be entirely optional / we could just warn here
        let source_watcher = source_reader.watch_for_changes(source_event_sender);
        if source_watcher.is_none() {
            error!("{}", CANNOT_WATCH_ERROR_MESSAGE);
        }
        AssetProcessorData {
            source_reader,
            source_writer,
            destination_reader,
            destination_writer,
            finished_sender,
            finished_receiver,
            initialized_sender,
            initialized_receiver,
            source_event_receiver,
            _source_watcher: source_watcher,
            state: async_lock::RwLock::new(ProcessorState::Initializing),
            log: Default::default(),
            processors: Default::default(),
            asset_infos: Default::default(),
            default_processors: Default::default(),
        }
    }

    /// Returns a future that will not finish until the path has been processed.
    pub async fn wait_until_processed(&self, path: &Path) -> ProcessStatus {
        self.wait_until_initialized().await;
        let mut receiver = {
            let infos = self.asset_infos.write().await;
            let info = infos.get(&AssetPath::from_path(path.to_path_buf()));
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

/// The (successful) result of processing an asset
#[derive(Debug, Clone)]
pub enum ProcessResult {
    Processed(ProcessedInfo),
    SkippedNotChanged,
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
    dependants: HashSet<AssetPath<'static>>,
    status: Option<ProcessStatus>,
    /// A lock that controls read/write access to processed asset files. The lock is shared for both the asset bytes and the meta bytes.
    /// _This lock must be locked whenever a read or write to processed assets occurs_
    /// There are scenarios where processed assets (and their metadata) are being read and written in multiple places at once:
    /// * when the processor is running in parallel with an app
    /// * when processing assets in parallel, the processor might read an asset's process_dependencies when processing new versions of those dependencies
    ///     * this second scenario almost certainly isn't possible with the current implementation, but its worth protecting against
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
            dependants: Default::default(),
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
    /// non_existent_dependants DATA IS CONSUMED
    infos: HashMap<AssetPath<'static>, ProcessorAssetInfo>,
    /// Dependants for assets that don't exist. This exists to track "dangling" asset references due to deleted / missing files.
    /// If the dependant asset is added, it can "resolve" these dependencies and re-compute those assets.
    /// Therefore this _must_ always be consistent with the `infos` data. If a new asset is added to `infos`, it should
    /// check this maps for dependencies and add them. If an asset is removed, it should update the dependants here.
    non_existent_dependants: HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
    check_reprocess_queue: VecDeque<PathBuf>,
}

impl ProcessorAssetInfos {
    fn get_or_insert(&mut self, asset_path: AssetPath<'static>) -> &mut ProcessorAssetInfo {
        self.infos.entry(asset_path.clone()).or_insert_with(|| {
            let mut info = ProcessorAssetInfo::default();
            // track existing dependants by resolving existing "hanging" dependants.
            if let Some(dependants) = self.non_existent_dependants.remove(&asset_path) {
                info.dependants = dependants;
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

    fn add_dependant(&mut self, asset_path: &AssetPath<'static>, dependant: AssetPath<'static>) {
        if let Some(info) = self.get_mut(asset_path) {
            info.dependants.insert(dependant);
        } else {
            let dependants = self
                .non_existent_dependants
                .entry(asset_path.clone())
                .or_default();
            dependants.insert(dependant);
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
                debug!("Finished processing \"{:?}\"", asset_path);
                // clean up old dependants
                let old_processed_info = self
                    .infos
                    .get_mut(&asset_path)
                    .and_then(|i| i.processed_info.take());
                if let Some(old_processed_info) = old_processed_info {
                    self.clear_dependencies(&asset_path, old_processed_info);
                }

                // populate new dependants
                for process_dependency_info in &processed_info.process_dependencies {
                    self.add_dependant(&process_dependency_info.path, asset_path.to_owned());
                }
                let info = self.get_or_insert(asset_path);
                info.processed_info = Some(processed_info);
                info.update_status(ProcessStatus::Processed).await;
                let dependants = info.dependants.iter().cloned().collect::<Vec<_>>();
                for path in dependants {
                    self.check_reprocess_queue.push_back(path.path().to_owned());
                }
            }
            Ok(ProcessResult::SkippedNotChanged) => {
                debug!("Skipping processing (unchanged) \"{:?}\"", asset_path);
                let info = self.get_mut(&asset_path).expect("info should exist");
                // NOTE: skipping an asset on a given pass doesn't mean it won't change in the future as a result
                // of a dependency being re-processed. This means apps might receive an "old" (but valid) asset first.
                // This is in the interest of fast startup times that don't block for all assets being checked + reprocessed
                // Therefore this relies on hot-reloading in the app to pickup the "latest" version of the asset
                // If "block until latest state is reflected" is required, we can easily add a less granular
                // "block until first pass finished" mode
                info.update_status(ProcessStatus::Processed).await;
            }
            Err(ProcessError::ExtensionRequired) => {
                // Skip assets without extensions
            }
            Err(ProcessError::MissingAssetLoaderForExtension(_)) => {
                trace!("No loader found for {:?}", asset_path);
            }
            Err(ProcessError::MissingAssetSource(_)) => {
                // if there is no asset source, no processing can be done
                trace!(
                    "No need to process asset {:?} because it does not exist",
                    asset_path
                );
            }
            Err(err) => {
                error!("Failed to process asset {:?}: {:?}", asset_path, err);
                // if this failed because a dependency could not be loaded, make sure it is reprocessed if that dependency is reprocessed
                if let ProcessError::AssetLoadError(AssetLoadError::AssetLoaderError {
                    error: AssetLoaderError::Load(loader_error),
                    ..
                }) = err
                {
                    if let Some(error) = loader_error.downcast_ref::<LoadDirectError>() {
                        let info = self.get_mut(&asset_path).expect("info should exist");
                        info.processed_info = Some(ProcessedInfo {
                            hash: AssetHash::default(),
                            full_hash: AssetHash::default(),
                            process_dependencies: vec![],
                        });
                        self.add_dependant(&error.dependency, asset_path.to_owned());
                    }
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
            if !info.dependants.is_empty() {
                error!(
                    "The asset at {asset_path} was removed, but it had assets that depend on it to be processed. Consider updating the path in the following assets: {:?}",
                    info.dependants
                );
                self.non_existent_dependants
                    .insert(asset_path.clone(), info.dependants);
            }
        }
    }

    /// Remove the info for the given path. This should only happen if an asset's source is removed / non-existent
    async fn rename(&mut self, old: &AssetPath<'static>, new: &AssetPath<'static>) {
        let info = self.infos.remove(old);
        if let Some(mut info) = info {
            if !info.dependants.is_empty() {
                // TODO: We can't currently ensure "moved" folders with relative paths aren't broken because AssetPath
                // doesn't distinguish between absolute and relative paths. We have "erased" relativeness. In the short term,
                // we could do "remove everything in a folder and re-add", but that requires full rebuilds / destroying the cache.
                // If processors / loaders could enumerate dependencies, we could check if the new deps line up with a rename.
                // If deps encoded "relativeness" as part of loading, that would also work (this seems like the right call).
                // TODO: it would be nice to log an error here for dependants that aren't also being moved + fixed.
                // (see the remove impl).
                error!(
                    "The asset at {old} was removed, but it had assets that depend on it to be processed. Consider updating the path in the following assets: {:?}",
                    info.dependants
                );
                self.non_existent_dependants
                    .insert(old.clone(), std::mem::take(&mut info.dependants));
            }
            if let Some(processed_info) = &info.processed_info {
                // Update "dependant" lists for this asset's "process dependencies" to use new path.
                for dep in &processed_info.process_dependencies {
                    if let Some(info) = self.infos.get_mut(&dep.path) {
                        info.dependants.remove(old);
                        info.dependants.insert(new.clone());
                    } else if let Some(dependants) = self.non_existent_dependants.get_mut(&dep.path)
                    {
                        dependants.remove(old);
                        dependants.insert(new.clone());
                    }
                }
            }
            // Tell all listeners this asset no longer exists
            info.status_sender
                .broadcast(ProcessStatus::NonExistent)
                .await
                .unwrap();
            let dependants: Vec<AssetPath<'static>> = {
                let new_info = self.get_or_insert(new.clone());
                new_info.processed_info = info.processed_info;
                new_info.status = info.status;
                // Ensure things waiting on the new path are informed of the status of this asset
                if let Some(status) = new_info.status {
                    new_info.status_sender.broadcast(status).await.unwrap();
                }
                new_info.dependants.iter().cloned().collect()
            };
            // Queue the asset for a reprocess check, in case it needs new meta.
            self.check_reprocess_queue.push_back(new.path().to_owned());
            for dependant in dependants {
                // Queue dependants for reprocessing because they might have been waiting for this asset.
                self.check_reprocess_queue.push_back(dependant.into());
            }
        }
    }

    fn clear_dependencies(&mut self, asset_path: &AssetPath<'static>, removed_info: ProcessedInfo) {
        for old_load_dep in removed_info.process_dependencies {
            if let Some(info) = self.infos.get_mut(&old_load_dep.path) {
                info.dependants.remove(asset_path);
            } else if let Some(dependants) =
                self.non_existent_dependants.get_mut(&old_load_dep.path)
            {
                dependants.remove(asset_path);
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
    ValidateLogError(ValidateLogError),
}
