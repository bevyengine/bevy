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
        AssetMetaDyn, AssetMetaMinimal, AssetMetaProcessedInfoMinimal, ProcessedInfo,
    },
    AssetLoadError, AssetLoaderError, AssetPath, AssetServer, DeserializeMetaError,
    LoadDirectError, MissingAssetLoaderForExtensionError,
};
use bevy_ecs::prelude::*;
use bevy_log::{debug, error, trace, warn};
use bevy_tasks::{IoTaskPool, Scope};
use bevy_utils::{BoxedFuture, HashMap, HashSet};
use futures_io::ErrorKind;
use futures_lite::{AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt};
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use thiserror::Error;

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

    pub async fn get_state(&self) -> ProcessorState {
        *self.data.state.read().await
    }

    pub fn source_reader(&self) -> &dyn AssetReader {
        &*self.data.source_reader
    }

    pub fn source_writer(&self) -> &dyn AssetWriter {
        &*self.data.source_writer
    }

    pub fn destination_reader(&self) -> &dyn AssetReader {
        &*self.data.destination_reader
    }

    pub fn destination_writer(&self) -> &dyn AssetWriter {
        &*self.data.destination_writer
    }

    pub fn start(processor: Res<Self>) {
        let processor = processor.clone();
        std::thread::spawn(move || {
            processor.process_assets();
            futures_lite::future::block_on(processor.listen_for_source_change_events());
        });
    }

    // TODO: document this process in full and describe why the "eventual consistency" works
    pub fn process_assets(&self) {
        let start_time = Instant::now();
        debug!("Processing started");
        IoTaskPool::get().scope(|scope| {
            scope.spawn(async move {
                self.initialize().await.unwrap();
                let path = PathBuf::from("");
                self.process_assets_internal(scope, path).await.unwrap();
            });
        });
        // This must happen _after_ the scope resolves or it will happen "too early"
        // Don't move this into the async scope above! process_assets is a blocking/sync function this is fine
        futures_lite::future::block_on(self.finish_processing_assets());
        let end_time = Instant::now();
        debug!("Processing finished in {:?}", end_time - start_time);
    }

    // PERF: parallelize change event processing
    pub async fn listen_for_source_change_events(&self) {
        debug!("Listening for changes to source assets");
        loop {
            let mut started_processing = false;
            for event in self.data.source_event_receiver.try_iter() {
                if !started_processing {
                    // TODO: re-enable this after resolving state change signaling issue
                    // self.set_state(ProcessorState::Processing).await;
                    started_processing = true;
                }
                match event {
                    AssetSourceEvent::Added(path)
                    | AssetSourceEvent::AddedMeta(path)
                    | AssetSourceEvent::Modified(path)
                    | AssetSourceEvent::ModifiedMeta(path) => {
                        debug!("Asset {:?} was modified. Attempting to re-process", path);
                        self.process_asset(&path).await;
                    }
                    AssetSourceEvent::Removed(path) => {
                        debug!("Removing processed {:?} because source was removed", path);
                        error!("remove is not implemented");
                        // // TODO: clean up in memory
                        // if let Err(err) = self.destination_writer().remove(&path).await {
                        //     warn!("Failed to remove non-existent asset {path:?}: {err}");
                        // }
                    }
                    AssetSourceEvent::RemovedMeta(path) => {
                        // If meta was removed, we might need to regenerate it.
                        // Likewise, the user might be manually re-adding the asset.
                        // Therefore, we shouldn't automatically delete meta ... that is a
                        // user-initiated action.
                        debug!(
                            "Meta for asset {:?} was removed. Attempting to re-process",
                            path
                        );
                        self.process_asset(&path).await;
                    }
                    AssetSourceEvent::AddedFolder(path) => {
                        debug!("Folder {:?} was added. Attempting to re-process", path);
                        // error!("add folder not implemented");
                        IoTaskPool::get().scope(|scope| {
                            scope.spawn(async move {
                                self.process_assets_internal(scope, path).await.unwrap();
                            });
                        });
                    }
                    AssetSourceEvent::RemovedFolder(path) => {
                        debug!("Removing folder {:?} because source was removed", path);
                        error!("remove folder is not implemented");
                        // TODO: clean up memory
                        // if let Err(err) = self.destination_writer().remove_directory(&path).await {
                        //     warn!("Failed to remove folder {path:?}: {err}");
                        // }
                    }
                }
            }

            if started_processing {
                self.finish_processing_assets().await;
            }
        }
    }

    async fn finish_processing_assets(&self) {
        self.try_reprocessing_queued().await;
        // clean up metadata in asset server
        self.server.data.infos.write().consume_handle_drop_events();
        self.set_state(ProcessorState::Finished).await;
    }

    fn process_assets_internal<'scope>(
        &'scope self,
        scope: &'scope Scope<'scope, '_, ()>,
        path: PathBuf,
    ) -> bevy_utils::BoxedFuture<'scope, Result<(), AssetReaderError>> {
        async move {
            if self.source_reader().is_directory(&path).await? {
                let mut path_stream = self.source_reader().read_directory(&path).await.unwrap();
                while let Some(path) = path_stream.next().await {
                    self.process_assets_internal(scope, path).await?;
                }
            } else {
                // Files without extensions are skipped
                if path.extension().is_some() {
                    let processor = self.clone();
                    scope.spawn(async move {
                        processor.process_asset(&path).await;
                    });
                }
            }
            Ok(())
        }
        .boxed()
    }

    async fn try_reprocessing_queued(&self) {
        let mut check_reprocess = true;
        while check_reprocess {
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
            check_reprocess = !infos.check_reprocess_queue.is_empty();
        }
    }

    pub fn register_processor<P: Process>(&self, processor: P) {
        let mut process_plans = self.data.processors.write();
        process_plans.insert(std::any::type_name::<P>(), Arc::new(processor));
    }

    pub fn set_default_processor<P: Process>(&self, extension: &str) {
        let mut default_processors = self.data.default_processors.write();
        default_processors.insert(extension.to_string(), std::any::type_name::<P>());
    }

    pub fn get_default_processor(&self, extension: &str) -> Option<Arc<dyn ErasedProcessor>> {
        let default_processors = self.data.default_processors.read();
        let key = default_processors.get(extension)?;
        self.data.processors.read().get(key).cloned()
    }

    pub fn get_processor(&self, processor_type_name: &str) -> Option<Arc<dyn ErasedProcessor>> {
        let processors = self.data.processors.read();
        processors.get(processor_type_name).cloned()
    }

    /// Populates the initial view of each asset by scanning the source and destination folders.
    /// This info will later be used to determine whether or not to re-process an asset
    async fn initialize(&self) -> Result<(), InitializeError> {
        self.validate_transaction_log_and_recover().await;
        let mut asset_infos = self.data.asset_infos.write().await;
        fn get_asset_paths<'a>(
            reader: &'a dyn AssetReader,
            path: PathBuf,
            paths: &'a mut Vec<PathBuf>,
        ) -> BoxedFuture<'a, Result<(), AssetReaderError>> {
            async move {
                if reader.is_directory(&path).await? {
                    let mut path_stream = reader.read_directory(&path).await?;
                    while let Some(child_path) = path_stream.next().await {
                        get_asset_paths(reader, child_path, paths).await?;
                    }
                } else {
                    paths.push(path);
                }
                Ok(())
            }
            .boxed()
        }

        let mut source_paths = Vec::new();
        let source_reader = self.source_reader();
        get_asset_paths(source_reader, PathBuf::from(""), &mut source_paths)
            .await
            .map_err(InitializeError::FailedToReadSourcePaths)?;

        let mut destination_paths = Vec::new();
        let destination_reader = self.destination_reader();
        get_asset_paths(
            destination_reader,
            PathBuf::from(""),
            &mut destination_paths,
        )
        .await
        .map_err(InitializeError::FailedToReadSourcePaths)?;

        for path in &source_paths {
            asset_infos.get_or_insert(AssetPath::new(path.to_owned(), None));
        }

        for path in &destination_paths {
            let asset_path = AssetPath::new(path.to_owned(), None);
            let mut dependencies = Vec::new();
            if let Some(info) = asset_infos.get_mut(&asset_path) {
                match self.destination_reader().read_meta_bytes(path).await {
                    Ok(meta_bytes) => {
                        match ron::de::from_bytes::<AssetMetaProcessedInfoMinimal>(&meta_bytes) {
                            Ok(minimal) => {
                                debug!(
                                    "Populated processed info for asset {path:?} {:?}",
                                    minimal.processed_info
                                );

                                if let Some(processed_info) = &minimal.processed_info {
                                    for process_dependency_info in
                                        &processed_info.process_dependencies
                                    {
                                        dependencies.push(process_dependency_info.path.to_owned());
                                    }
                                }
                                info.processed_info = minimal.processed_info;
                            }
                            Err(err) => {
                                debug!("Removing processed data for {path:?} because meta could not be parsed: {err}");
                                self.remove_processed_asset(path).await;
                            }
                        }
                    }
                    Err(err) => {
                        debug!("Removing processed data for {path:?} because meta failed to load: {err}");
                        self.remove_processed_asset(path).await;
                    }
                }
            } else {
                debug!("Removing processed data for non-existent asset {path:?}");
                self.remove_processed_asset(path).await;
            }

            for dependency in dependencies {
                asset_infos.add_dependant(&dependency, asset_path.to_owned());
            }
        }

        self.set_state(ProcessorState::Processing).await;

        Ok(())
    }

    async fn remove_processed_asset(&self, path: &Path) {
        if let Err(err) = self.destination_writer().remove(path).await {
            warn!("Failed to remove non-existent asset {path:?}: {err}");
        }

        if let Err(err) = self.destination_writer().remove_meta(path).await {
            warn!("Failed to remove non-existent meta {path:?}: {err}");
        }
    }

    async fn process_asset(&self, path: &Path) {
        let result = self.process_asset_internal(path).await;
        let mut infos = self.data.asset_infos.write().await;
        let asset_path = AssetPath::new(path.to_owned(), None);
        infos.finish_processing(asset_path, result).await;
    }

    async fn process_asset_internal(&self, path: &Path) -> Result<ProcessResult, ProcessError> {
        let asset_path = AssetPath::new(path.to_owned(), None);
        // TODO: check if already processing to protect against duplicate hot-reload events
        debug!("Processing asset {:?}", path);
        let server = &self.server;
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
                        let loader = server.get_asset_loader_with_type_name(&loader)?;
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
                    match server.get_path_asset_loader(&asset_path) {
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
                let mut meta_writer = self.source_writer().write_meta(path).await?;
                // TODO: handle error
                meta_writer.write_all(&meta_bytes).await.unwrap();
                meta_writer.flush().await.unwrap();
                (meta, meta_bytes, processor)
            }
            Err(err) => return Err(ProcessError::ReadAssetMetaError(err)),
        };

        // TODO:  check timestamp first for early-out
        // TODO: error handling
        let mut reader = self.source_reader().read(path).await.unwrap();
        let mut asset_bytes = Vec::new();
        reader.read_to_end(&mut asset_bytes).await.unwrap();
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
            info.file_transaction_lock.write_arc()
        };

        let mut writer = self.destination_writer().write(path).await?;
        let mut meta_writer = self.destination_writer().write_meta(path).await?;
        // NOTE: if processing the asset fails this will produce an "unfinished" log entry, forcing a rebuild on next run.
        // Directly writing to the asset destination in the processor necessitates this behavior
        // TODO: this class of failure can be recovered via re-processing + smarter log validation that allows for duplicate transactions in the event of failures
        {
            let mut logger = self.data.log.write().await;
            logger.as_mut().unwrap().begin_path(path).await.unwrap();
        }

        if let Some(processor) = processor {
            let mut processed_meta = {
                let mut context =
                    ProcessContext::new(self, &asset_path, &asset_bytes, &mut new_processed_info);
                processor
                    .process(&mut context, source_meta, &mut *writer)
                    .await?
            };

            // TODO: error handling
            writer.flush().await.unwrap();

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
            meta_writer.write_all(&meta_bytes).await.unwrap();
            meta_writer.flush().await.unwrap();
        } else {
            // TODO: error handling
            writer.write_all(&asset_bytes).await.unwrap();
            writer.flush().await.unwrap();
            *source_meta.processed_info_mut() = Some(new_processed_info.clone());
            let meta_bytes = source_meta.serialize();
            meta_writer.write_all(&meta_bytes).await.unwrap();
            meta_writer.flush().await.unwrap();
        }

        {
            let mut logger = self.data.log.write().await;
            logger.as_mut().unwrap().end_path(path).await.unwrap();
        }

        Ok(ProcessResult::Processed(new_processed_info))
    }

    async fn validate_transaction_log_and_recover(&self) {
        if let Err(err) = ProcessorTransactionLog::validate().await {
            let state_is_valid = match err {
                ValidateLogError::ReadLogError(err) => {
                    error!("Failed to read processor log file. Processed assets cannot be validated so they must be re-generated {err}");
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
                                debug!("Asset {path:?} did not finish processing. Clearning state for that asset");
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
                    panic!("Processed assets were in a bad state. To correct this, the asset processor attempted to remove all processed assets and start from scratch. This failed. There is no way to continue. Try restarting, or deleting imported asset state manually. {err}");
                }
            }
        }
        let mut log = self.data.log.write().await;
        *log = match ProcessorTransactionLog::new().await {
            Ok(log) => Some(log),
            Err(err) => panic!("Failed to initialize asset processor log. This cannot be recovered. Try restarting. If that doesn't work, try deleting processed asset state. {}", err),
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
        let (finished_sender, finished_receiver) = async_broadcast::broadcast(1);
        let (initialized_sender, initialized_receiver) = async_broadcast::broadcast(1);
        let (source_event_sender, source_event_receiver) = crossbeam_channel::unbounded();
        // TODO: watching for changes could probably be entirely optional / we could just warn here
        let source_watcher = source_reader.watch_for_changes(source_event_sender);
        if source_watcher.is_none() {
            error!(
                "Cannot watch for changes because the current `AssetReader` does not support it"
            );
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

    pub async fn wait_until_processed(&self, path: &Path) -> ProcessStatus {
        self.wait_until_initialized().await;
        let mut receiver = {
            let infos = self.asset_infos.write().await;
            let info = infos.get(&AssetPath::new(path.to_owned(), None));
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

#[derive(Debug, Clone)]
pub enum ProcessResult {
    Processed(ProcessedInfo),
    SkippedNotChanged,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ProcessStatus {
    Processed,
    Failed,
    NonExistent,
}

pub(crate) struct ProcessorAssetInfo {
    processed_info: Option<ProcessedInfo>,
    dependants: HashSet<AssetPath<'static>>,
    status: Option<ProcessStatus>,
    /// A lock that controls read/write access to processed asset files. The lock is shared for both the asset bytes and the meta bytes.
    /// _This lock must be locked whenever a read or write to processed assets occurs_
    /// There are scenarios where processed assets (and their metadata) are being read and written in multiple places at once:
    /// * when the processor is running in parallel with an app
    /// * when processing assets in parallel, the processor might read an asset's process_dependencies when processing new versions of those dependencies
    ///     * this second scenario almost certainly isn't possible with the current implementation, but its worth protecting against
    /// This lock defends against those scenarios by ensuring readers don't read while processed files are being written. And it ensures
    /// Because this lock is shared across meta and asset bytes, readers can esure they don't read "old" versions of metadata with "new" asset data.  
    pub(crate) file_transaction_lock: Arc<RwLock<()>>,
    status_sender: async_broadcast::Sender<ProcessStatus>,
    status_receiver: async_broadcast::Receiver<ProcessStatus>,
}

impl Default for ProcessorAssetInfo {
    fn default() -> Self {
        let (status_sender, status_receiver) = async_broadcast::broadcast(1);
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
#[derive(Default)]
pub struct ProcessorAssetInfos {
    /// The "current" in memory view of the asset space. During processing, if path does not exist in this, it should
    /// be considered non-existent.
    /// NOTE: YOU MUST USE `get_or_insert` TO ADD ITEMS TO THIS COLLECTION
    infos: HashMap<AssetPath<'static>, ProcessorAssetInfo>,
    /// Dependants for assets that don't exist. This exists to track "dangling" asset references due to deleted / missing files.
    /// If the dependant asset is added, it can "resolve" these dependancies and re-compute those assets.
    /// Therefore this _must_ always be consistent with the `infos` data. If a new asset is added to `infos`, it should
    /// check this maps for dependencies and add them. If an asset is removed, it should update the dependants here.
    non_existent_dependants: HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
    check_reprocess_queue: VecDeque<PathBuf>,
}

impl ProcessorAssetInfos {
    fn get_or_insert(&mut self, asset_path: AssetPath<'static>) -> &mut ProcessorAssetInfo {
        self.infos.entry(asset_path.clone()).or_insert_with(|| {
            let mut info = ProcessorAssetInfo::default();
            // track existing dependenants by resolving existing "hanging" dependants.
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
                .entry(asset_path.to_owned())
                .or_default();
            dependants.insert(dependant);
        }
    }

    async fn finish_processing(
        &mut self,
        asset_path: AssetPath<'static>,
        result: Result<ProcessResult, ProcessError>,
    ) {
        match result {
            Ok(ProcessResult::Processed(processed_info)) => {
                debug!("Finished processing asset {:?}", asset_path,);
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
                debug!(
                    "Skipping processing of asset {:?} because it has not changed",
                    asset_path
                );
                let info = self.get_mut(&asset_path).expect("info should exist");
                // NOTE: skipping an asset on a given pass doesn't mean it won't change in the future as a result
                // of a dependency being re-processed. This means apps might receive an "old" (but valid) asset first.
                // This is in the interest of fast startup times that don't block for all assets being checked + reprocessed
                // Therefore this relies on hot-reloading in the app to pickup the "latest" version of the asset
                // If "block until latest state is reflected" is required, we can easily add a less granular
                // "block until first pass finished" mode
                info.update_status(ProcessStatus::Processed).await;
            }
            Err(ProcessError::MissingAssetLoaderForExtension(_)) => {
                trace!("No loader found for {:?}", asset_path);
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

    // Remove the info for the given path. This should only happen if an asset's source is removed / non-existent
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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ProcessorState {
    Initializing,
    Processing,
    Finished,
}

#[derive(Error, Debug)]
pub enum InitializeError {
    #[error(transparent)]
    FailedToReadSourcePaths(AssetReaderError),
    #[error(transparent)]
    FailedToReadDestinationPaths(AssetReaderError),
    #[error("Failed to validate asset log: {0}")]
    ValidateLogError(ValidateLogError),
}
