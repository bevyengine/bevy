use crate::{
    io::{processor_gated::ProcessorGatedReader, AssetSourceEvent, AssetWatcher},
    processor::AssetProcessorData,
};
use bevy_ecs::system::Resource;
use bevy_utils::tracing::{error, warn};
use bevy_utils::{CowArc, Duration, HashMap};
use std::{fmt::Display, hash::Hash, sync::Arc};
use thiserror::Error;

use super::{ErasedAssetReader, ErasedAssetWriter};

// Needed for doc strings.
#[allow(unused_imports)]
use crate::io::{AssetReader, AssetWriter};

/// A reference to an "asset source", which maps to an [`AssetReader`] and/or [`AssetWriter`].
///
/// * [`AssetSourceId::Default`] corresponds to "default asset paths" that don't specify a source: `/path/to/asset.png`
/// * [`AssetSourceId::Name`] corresponds to asset paths that _do_ specify a source: `remote://path/to/asset.png`, where `remote` is the name.
#[derive(Default, Clone, Debug, Eq)]
pub enum AssetSourceId<'a> {
    /// The default asset source.
    #[default]
    Default,
    /// A non-default named asset source.
    Name(CowArc<'a, str>),
}

impl<'a> Display for AssetSourceId<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_str() {
            None => write!(f, "AssetSourceId::Default"),
            Some(v) => write!(f, "AssetSourceId::Name({v})"),
        }
    }
}

impl<'a> AssetSourceId<'a> {
    /// Creates a new [`AssetSourceId`]
    pub fn new(source: Option<impl Into<CowArc<'a, str>>>) -> AssetSourceId<'a> {
        match source {
            Some(source) => AssetSourceId::Name(source.into()),
            None => AssetSourceId::Default,
        }
    }

    /// Returns [`None`] if this is [`AssetSourceId::Default`] and [`Some`] containing the
    /// name if this is [`AssetSourceId::Name`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AssetSourceId::Default => None,
            AssetSourceId::Name(v) => Some(v),
        }
    }

    /// If this is not already an owned / static id, create one. Otherwise, it will return itself (with a static lifetime).
    pub fn into_owned(self) -> AssetSourceId<'static> {
        match self {
            AssetSourceId::Default => AssetSourceId::Default,
            AssetSourceId::Name(v) => AssetSourceId::Name(v.into_owned()),
        }
    }

    /// Clones into an owned [`AssetSourceId<'static>`].
    /// This is equivalent to `.clone().into_owned()`.
    #[inline]
    pub fn clone_owned(&self) -> AssetSourceId<'static> {
        self.clone().into_owned()
    }
}

impl From<&'static str> for AssetSourceId<'static> {
    fn from(value: &'static str) -> Self {
        AssetSourceId::Name(value.into())
    }
}

impl<'a, 'b> From<&'a AssetSourceId<'b>> for AssetSourceId<'b> {
    fn from(value: &'a AssetSourceId<'b>) -> Self {
        value.clone()
    }
}

impl From<Option<&'static str>> for AssetSourceId<'static> {
    fn from(value: Option<&'static str>) -> Self {
        match value {
            Some(value) => AssetSourceId::Name(value.into()),
            None => AssetSourceId::Default,
        }
    }
}

impl From<String> for AssetSourceId<'static> {
    fn from(value: String) -> Self {
        AssetSourceId::Name(value.into())
    }
}

impl<'a> Hash for AssetSourceId<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl<'a> PartialEq for AssetSourceId<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(&other.as_str())
    }
}

/// Metadata about an "asset source", such as how to construct the [`AssetReader`] and [`AssetWriter`] for the source,
/// and whether or not the source is processed.
#[derive(Default)]
pub struct AssetSourceBuilder {
    pub reader: Option<Box<dyn FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync>>,
    pub writer: Option<Box<dyn FnMut(bool) -> Option<Box<dyn ErasedAssetWriter>> + Send + Sync>>,
    pub watcher: Option<
        Box<
            dyn FnMut(crossbeam_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
                + Send
                + Sync,
        >,
    >,
    pub processed_reader: Option<Box<dyn FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync>>,
    pub processed_writer:
        Option<Box<dyn FnMut(bool) -> Option<Box<dyn ErasedAssetWriter>> + Send + Sync>>,
    pub processed_watcher: Option<
        Box<
            dyn FnMut(crossbeam_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
                + Send
                + Sync,
        >,
    >,
    pub watch_warning: Option<&'static str>,
    pub processed_watch_warning: Option<&'static str>,
}

impl AssetSourceBuilder {
    /// Builds a new [`AssetSource`] with the given `id`. If `watch` is true, the unprocessed source will watch for changes.
    /// If `watch_processed` is true, the processed source will watch for changes.
    pub fn build(
        &mut self,
        id: AssetSourceId<'static>,
        watch: bool,
        watch_processed: bool,
    ) -> Option<AssetSource> {
        let reader = self.reader.as_mut()?();
        let writer = self.writer.as_mut().and_then(|w| w(false));
        let processed_writer = self.processed_writer.as_mut().and_then(|w| w(true));
        let mut source = AssetSource {
            id: id.clone(),
            reader,
            writer,
            processed_reader: self.processed_reader.as_mut().map(|r| r()),
            processed_writer,
            event_receiver: None,
            watcher: None,
            processed_event_receiver: None,
            processed_watcher: None,
        };

        if watch {
            let (sender, receiver) = crossbeam_channel::unbounded();
            match self.watcher.as_mut().and_then(|w| w(sender)) {
                Some(w) => {
                    source.watcher = Some(w);
                    source.event_receiver = Some(receiver);
                }
                None => {
                    if let Some(warning) = self.watch_warning {
                        warn!("{id} does not have an AssetWatcher configured. {warning}");
                    }
                }
            }
        }

        if watch_processed {
            let (sender, receiver) = crossbeam_channel::unbounded();
            match self.processed_watcher.as_mut().and_then(|w| w(sender)) {
                Some(w) => {
                    source.processed_watcher = Some(w);
                    source.processed_event_receiver = Some(receiver);
                }
                None => {
                    if let Some(warning) = self.processed_watch_warning {
                        warn!("{id} does not have a processed AssetWatcher configured. {warning}");
                    }
                }
            }
        }
        Some(source)
    }

    /// Will use the given `reader` function to construct unprocessed [`AssetReader`] instances.
    pub fn with_reader(
        mut self,
        reader: impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.reader = Some(Box::new(reader));
        self
    }

    /// Will use the given `writer` function to construct unprocessed [`AssetWriter`] instances.
    pub fn with_writer(
        mut self,
        writer: impl FnMut(bool) -> Option<Box<dyn ErasedAssetWriter>> + Send + Sync + 'static,
    ) -> Self {
        self.writer = Some(Box::new(writer));
        self
    }

    /// Will use the given `watcher` function to construct unprocessed [`AssetWatcher`] instances.
    pub fn with_watcher(
        mut self,
        watcher: impl FnMut(crossbeam_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.watcher = Some(Box::new(watcher));
        self
    }

    /// Will use the given `reader` function to construct processed [`AssetReader`] instances.
    pub fn with_processed_reader(
        mut self,
        reader: impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.processed_reader = Some(Box::new(reader));
        self
    }

    /// Will use the given `writer` function to construct processed [`AssetWriter`] instances.
    pub fn with_processed_writer(
        mut self,
        writer: impl FnMut(bool) -> Option<Box<dyn ErasedAssetWriter>> + Send + Sync + 'static,
    ) -> Self {
        self.processed_writer = Some(Box::new(writer));
        self
    }

    /// Will use the given `watcher` function to construct processed [`AssetWatcher`] instances.
    pub fn with_processed_watcher(
        mut self,
        watcher: impl FnMut(crossbeam_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.processed_watcher = Some(Box::new(watcher));
        self
    }

    /// Enables a warning for the unprocessed source watcher, which will print when watching is enabled and the unprocessed source doesn't have a watcher.
    pub fn with_watch_warning(mut self, warning: &'static str) -> Self {
        self.watch_warning = Some(warning);
        self
    }

    /// Enables a warning for the processed source watcher, which will print when watching is enabled and the processed source doesn't have a watcher.
    pub fn with_processed_watch_warning(mut self, warning: &'static str) -> Self {
        self.processed_watch_warning = Some(warning);
        self
    }

    /// Returns a builder containing the "platform default source" for the given `path` and `processed_path`.
    /// For most platforms, this will use [`FileAssetReader`](crate::io::file::FileAssetReader) / [`FileAssetWriter`](crate::io::file::FileAssetWriter),
    /// but some platforms (such as Android) have their own default readers / writers / watchers.
    pub fn platform_default(path: &str, processed_path: Option<&str>) -> Self {
        let default = Self::default()
            .with_reader(AssetSource::get_default_reader(path.to_string()))
            .with_writer(AssetSource::get_default_writer(path.to_string()))
            .with_watcher(AssetSource::get_default_watcher(
                path.to_string(),
                Duration::from_millis(300),
            ))
            .with_watch_warning(AssetSource::get_default_watch_warning());
        if let Some(processed_path) = processed_path {
            default
                .with_processed_reader(AssetSource::get_default_reader(processed_path.to_string()))
                .with_processed_writer(AssetSource::get_default_writer(processed_path.to_string()))
                .with_processed_watcher(AssetSource::get_default_watcher(
                    processed_path.to_string(),
                    Duration::from_millis(300),
                ))
                .with_processed_watch_warning(AssetSource::get_default_watch_warning())
        } else {
            default
        }
    }
}

/// A [`Resource`] that hold (repeatable) functions capable of producing new [`AssetReader`] and [`AssetWriter`] instances
/// for a given asset source.
#[derive(Resource, Default)]
pub struct AssetSourceBuilders {
    sources: HashMap<CowArc<'static, str>, AssetSourceBuilder>,
    default: Option<AssetSourceBuilder>,
}

impl AssetSourceBuilders {
    /// Inserts a new builder with the given `id`
    pub fn insert(&mut self, id: impl Into<AssetSourceId<'static>>, source: AssetSourceBuilder) {
        match id.into() {
            AssetSourceId::Default => {
                self.default = Some(source);
            }
            AssetSourceId::Name(name) => {
                self.sources.insert(name, source);
            }
        }
    }

    /// Gets a mutable builder with the given `id`, if it exists.
    pub fn get_mut<'a, 'b>(
        &'a mut self,
        id: impl Into<AssetSourceId<'b>>,
    ) -> Option<&'a mut AssetSourceBuilder> {
        match id.into() {
            AssetSourceId::Default => self.default.as_mut(),
            AssetSourceId::Name(name) => self.sources.get_mut(&name.into_owned()),
        }
    }

    /// Builds a new [`AssetSources`] collection. If `watch` is true, the unprocessed sources will watch for changes.
    /// If `watch_processed` is true, the processed sources will watch for changes.
    pub fn build_sources(&mut self, watch: bool, watch_processed: bool) -> AssetSources {
        let mut sources = HashMap::new();
        for (id, source) in &mut self.sources {
            if let Some(data) = source.build(
                AssetSourceId::Name(id.clone_owned()),
                watch,
                watch_processed,
            ) {
                sources.insert(id.clone_owned(), data);
            }
        }

        AssetSources {
            sources,
            default: self
                .default
                .as_mut()
                .and_then(|p| p.build(AssetSourceId::Default, watch, watch_processed))
                .expect(MISSING_DEFAULT_SOURCE),
        }
    }

    /// Initializes the default [`AssetSourceBuilder`] if it has not already been set.
    pub fn init_default_source(&mut self, path: &str, processed_path: Option<&str>) {
        self.default
            .get_or_insert_with(|| AssetSourceBuilder::platform_default(path, processed_path));
    }
}

/// A collection of unprocessed and processed [`AssetReader`], [`AssetWriter`], and [`AssetWatcher`] instances
/// for a specific asset source, identified by an [`AssetSourceId`].
pub struct AssetSource {
    id: AssetSourceId<'static>,
    reader: Box<dyn ErasedAssetReader>,
    writer: Option<Box<dyn ErasedAssetWriter>>,
    processed_reader: Option<Box<dyn ErasedAssetReader>>,
    processed_writer: Option<Box<dyn ErasedAssetWriter>>,
    watcher: Option<Box<dyn AssetWatcher>>,
    processed_watcher: Option<Box<dyn AssetWatcher>>,
    event_receiver: Option<crossbeam_channel::Receiver<AssetSourceEvent>>,
    processed_event_receiver: Option<crossbeam_channel::Receiver<AssetSourceEvent>>,
}

impl AssetSource {
    /// Starts building a new [`AssetSource`].
    pub fn build() -> AssetSourceBuilder {
        AssetSourceBuilder::default()
    }

    /// Returns this source's id.
    #[inline]
    pub fn id(&self) -> AssetSourceId<'static> {
        self.id.clone()
    }

    /// Return's this source's unprocessed [`AssetReader`].
    #[inline]
    pub fn reader(&self) -> &dyn ErasedAssetReader {
        &*self.reader
    }

    /// Return's this source's unprocessed [`AssetWriter`], if it exists.
    #[inline]
    pub fn writer(&self) -> Result<&dyn ErasedAssetWriter, MissingAssetWriterError> {
        self.writer
            .as_deref()
            .ok_or_else(|| MissingAssetWriterError(self.id.clone_owned()))
    }

    /// Return's this source's processed [`AssetReader`], if it exists.
    #[inline]
    pub fn processed_reader(
        &self,
    ) -> Result<&dyn ErasedAssetReader, MissingProcessedAssetReaderError> {
        self.processed_reader
            .as_deref()
            .ok_or_else(|| MissingProcessedAssetReaderError(self.id.clone_owned()))
    }

    /// Return's this source's processed [`AssetWriter`], if it exists.
    #[inline]
    pub fn processed_writer(
        &self,
    ) -> Result<&dyn ErasedAssetWriter, MissingProcessedAssetWriterError> {
        self.processed_writer
            .as_deref()
            .ok_or_else(|| MissingProcessedAssetWriterError(self.id.clone_owned()))
    }

    /// Return's this source's unprocessed event receiver, if the source is currently watching for changes.
    #[inline]
    pub fn event_receiver(&self) -> Option<&crossbeam_channel::Receiver<AssetSourceEvent>> {
        self.event_receiver.as_ref()
    }

    /// Return's this source's processed event receiver, if the source is currently watching for changes.
    #[inline]
    pub fn processed_event_receiver(
        &self,
    ) -> Option<&crossbeam_channel::Receiver<AssetSourceEvent>> {
        self.processed_event_receiver.as_ref()
    }

    /// Returns true if the assets in this source should be processed.
    #[inline]
    pub fn should_process(&self) -> bool {
        self.processed_writer.is_some()
    }

    /// Returns a builder function for this platform's default [`AssetReader`]. `path` is the relative path to
    /// the asset root.
    pub fn get_default_reader(
        _path: String,
    ) -> impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync {
        move || {
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            return Box::new(super::file::FileAssetReader::new(&_path));
            #[cfg(target_arch = "wasm32")]
            return Box::new(super::wasm::HttpWasmAssetReader::new(&_path));
            #[cfg(target_os = "android")]
            return Box::new(super::android::AndroidAssetReader);
        }
    }

    /// Returns a builder function for this platform's default [`AssetWriter`]. `path` is the relative path to
    /// the asset root. This will return [`None`] if this platform does not support writing assets by default.
    pub fn get_default_writer(
        _path: String,
    ) -> impl FnMut(bool) -> Option<Box<dyn ErasedAssetWriter>> + Send + Sync {
        move |_create_root: bool| {
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            return Some(Box::new(super::file::FileAssetWriter::new(
                &_path,
                _create_root,
            )));
            #[cfg(any(target_arch = "wasm32", target_os = "android"))]
            return None;
        }
    }

    /// Returns the default non-existent [`AssetWatcher`] warning for the current platform.
    pub fn get_default_watch_warning() -> &'static str {
        #[cfg(target_arch = "wasm32")]
        return "Web does not currently support watching assets.";
        #[cfg(target_os = "android")]
        return "Android does not currently support watching assets.";
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(target_os = "android"),
            not(feature = "file_watcher")
        ))]
        return "Consider enabling the `file_watcher` feature.";
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(target_os = "android"),
            feature = "file_watcher"
        ))]
        return "Consider adding an \"assets\" directory.";
    }

    /// Returns a builder function for this platform's default [`AssetWatcher`]. `path` is the relative path to
    /// the asset root. This will return [`None`] if this platform does not support watching assets by default.
    /// `file_debounce_time` is the amount of time to wait (and debounce duplicate events) before returning an event.
    /// Higher durations reduce duplicates but increase the amount of time before a change event is processed. If the
    /// duration is set too low, some systems might surface events _before_ their filesystem has the changes.
    #[allow(unused)]
    pub fn get_default_watcher(
        path: String,
        file_debounce_wait_time: Duration,
    ) -> impl FnMut(crossbeam_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
           + Send
           + Sync {
        move |sender: crossbeam_channel::Sender<AssetSourceEvent>| {
            #[cfg(all(
                feature = "file_watcher",
                not(target_arch = "wasm32"),
                not(target_os = "android")
            ))]
            {
                let path = std::path::PathBuf::from(path.clone());
                if path.exists() {
                    Some(Box::new(
                        super::file::FileWatcher::new(
                            path.clone(),
                            sender,
                            file_debounce_wait_time,
                        )
                        .unwrap_or_else(|e| {
                            panic!("Failed to create file watcher from path {path:?}, {e:?}")
                        }),
                    ))
                } else {
                    warn!("Skip creating file watcher because path {path:?} does not exist.");
                    None
                }
            }
            #[cfg(any(
                not(feature = "file_watcher"),
                target_arch = "wasm32",
                target_os = "android"
            ))]
            return None;
        }
    }

    /// This will cause processed [`AssetReader`] futures (such as [`AssetReader::read`]) to wait until
    /// the [`AssetProcessor`](crate::AssetProcessor) has finished processing the requested asset.
    pub fn gate_on_processor(&mut self, processor_data: Arc<AssetProcessorData>) {
        if let Some(reader) = self.processed_reader.take() {
            self.processed_reader = Some(Box::new(ProcessorGatedReader::new(
                self.id(),
                reader,
                processor_data,
            )));
        }
    }
}

/// A collection of [`AssetSource`]s.
pub struct AssetSources {
    sources: HashMap<CowArc<'static, str>, AssetSource>,
    default: AssetSource,
}

impl AssetSources {
    /// Gets the [`AssetSource`] with the given `id`, if it exists.
    pub fn get<'a, 'b>(
        &'a self,
        id: impl Into<AssetSourceId<'b>>,
    ) -> Result<&'a AssetSource, MissingAssetSourceError> {
        match id.into().into_owned() {
            AssetSourceId::Default => Ok(&self.default),
            AssetSourceId::Name(name) => self
                .sources
                .get(&name)
                .ok_or_else(|| MissingAssetSourceError(AssetSourceId::Name(name))),
        }
    }

    /// Iterates all asset sources in the collection (including the default source).
    pub fn iter(&self) -> impl Iterator<Item = &AssetSource> {
        self.sources.values().chain(Some(&self.default))
    }

    /// Mutably iterates all asset sources in the collection (including the default source).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut AssetSource> {
        self.sources.values_mut().chain(Some(&mut self.default))
    }

    /// Iterates all processed asset sources in the collection (including the default source).
    pub fn iter_processed(&self) -> impl Iterator<Item = &AssetSource> {
        self.iter().filter(|p| p.should_process())
    }

    /// Mutably iterates all processed asset sources in the collection (including the default source).
    pub fn iter_processed_mut(&mut self) -> impl Iterator<Item = &mut AssetSource> {
        self.iter_mut().filter(|p| p.should_process())
    }

    /// Iterates over the [`AssetSourceId`] of every [`AssetSource`] in the collection (including the default source).
    pub fn ids(&self) -> impl Iterator<Item = AssetSourceId<'static>> + '_ {
        self.sources
            .keys()
            .map(|k| AssetSourceId::Name(k.clone_owned()))
            .chain(Some(AssetSourceId::Default))
    }

    /// This will cause processed [`AssetReader`] futures (such as [`AssetReader::read`]) to wait until
    /// the [`AssetProcessor`](crate::AssetProcessor) has finished processing the requested asset.
    pub fn gate_on_processor(&mut self, processor_data: Arc<AssetProcessorData>) {
        for source in self.iter_processed_mut() {
            source.gate_on_processor(processor_data.clone());
        }
    }
}

/// An error returned when an [`AssetSource`] does not exist for a given id.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Source '{0}' does not exist")]
pub struct MissingAssetSourceError(AssetSourceId<'static>);

/// An error returned when an [`AssetWriter`] does not exist for a given id.
#[derive(Error, Debug, Clone)]
#[error("Asset Source '{0}' does not have an AssetWriter.")]
pub struct MissingAssetWriterError(AssetSourceId<'static>);

/// An error returned when a processed [`AssetReader`] does not exist for a given id.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Source '{0}' does not have a processed AssetReader.")]
pub struct MissingProcessedAssetReaderError(AssetSourceId<'static>);

/// An error returned when a processed [`AssetWriter`] does not exist for a given id.
#[derive(Error, Debug, Clone)]
#[error("Asset Source '{0}' does not have a processed AssetWriter.")]
pub struct MissingProcessedAssetWriterError(AssetSourceId<'static>);

const MISSING_DEFAULT_SOURCE: &str =
    "A default AssetSource is required. Add one to `AssetSourceBuilders`";
