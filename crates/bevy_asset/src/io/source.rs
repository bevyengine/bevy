use crate::{
    io::{processor_gated::ProcessorGatedReader, AssetSourceEvent, AssetWatcher},
    processor::ProcessingState,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};
use atomicow::CowArc;
use bevy_ecs::resource::Resource;
use bevy_platform::collections::HashMap;
use core::{fmt::Display, hash::Hash, time::Duration};
use derive_more::{Deref, DerefMut};
use thiserror::Error;
use tracing::warn;

use super::{ErasedAssetReader, ErasedAssetWriter};

/// A reference to an "asset source", which maps to an [`AssetReader`](crate::io::AssetReader) and/or [`AssetWriter`](crate::io::AssetWriter).
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

// This is only implemented for static lifetimes to ensure `Path::clone` does not allocate
// by ensuring that this is stored as a `CowArc::Static`.
// Please read https://github.com/bevyengine/bevy/issues/19844 before changing this!
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
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl<'a> PartialEq for AssetSourceId<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(&other.as_str())
    }
}

/// Metadata about an "asset source", such as how to construct the [`AssetReader`](crate::io::AssetReader) and [`AssetWriter`](crate::io::AssetWriter) for the source,
/// and whether or not the source is processed.
pub struct AssetSourceBuilder {
    /// The [`ErasedAssetReader`] to use on the unprocessed asset.
    pub reader: Box<dyn FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync>,
    /// The [`ErasedAssetWriter`] to use on the unprocessed asset.
    pub writer: Option<Box<dyn FnMut(bool) -> Option<Box<dyn ErasedAssetWriter>> + Send + Sync>>,
    /// The [`AssetWatcher`] to use for unprocessed assets, if any.
    pub watcher: Option<
        Box<
            dyn FnMut(async_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
                + Send
                + Sync,
        >,
    >,
    /// The warning message to display when watching an unprocessed asset fails.
    pub watch_warning: Option<&'static str>,
}

impl AssetSourceBuilder {
    /// Creates a new builder, starting with the provided reader.
    pub fn new(
        reader: impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync + 'static,
    ) -> AssetSourceBuilder {
        Self {
            reader: Box::new(reader),
            writer: None,
            watcher: None,
            watch_warning: None,
        }
    }

    /// Builds a new [`AssetSource`] with the given `id`.
    ///
    /// If `watch` is true, the source will watch for changes. If `create_root_for_writer`, the
    /// source is told its writer should create the root directory (if it does not exist).
    pub fn build(
        &mut self,
        id: AssetSourceId<'static>,
        watch: bool,
        create_root_for_writer: bool,
    ) -> AssetSource {
        let reader = self.reader.as_mut()().into();
        let writer = self.writer.as_mut().and_then(|w| w(create_root_for_writer));
        let mut source = AssetSource {
            id: id.clone(),
            reader,
            writer,
            event_receiver: None,
            watcher: None,
        };

        if watch {
            let (sender, receiver) = async_channel::unbounded();
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

        source
    }

    /// Will use the given `reader` function to construct unprocessed [`AssetReader`](crate::io::AssetReader) instances.
    pub fn with_reader(
        mut self,
        reader: impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.reader = Box::new(reader);
        self
    }

    /// Will use the given `writer` function to construct unprocessed [`AssetWriter`](crate::io::AssetWriter) instances.
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
        watcher: impl FnMut(async_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.watcher = Some(Box::new(watcher));
        self
    }

    /// Enables a warning for the unprocessed source watcher, which will print when watching is enabled and the unprocessed source doesn't have a watcher.
    pub fn with_watch_warning(mut self, warning: &'static str) -> Self {
        self.watch_warning = Some(warning);
        self
    }

    /// Returns a builder containing the "platform default source" for the given `path` and `processed_path`.
    /// For most platforms, this will use [`FileAssetReader`](crate::io::file::FileAssetReader) / [`FileAssetWriter`](crate::io::file::FileAssetWriter),
    /// but some platforms (such as Android) have their own default readers / writers / watchers.
    pub fn platform_default(path: &str) -> Self {
        Self::new(AssetSource::get_default_reader(path.to_string()))
            .with_writer(AssetSource::get_default_writer(path.to_string()))
            .with_watcher(AssetSource::get_default_watcher(
                path.to_string(),
                Duration::from_millis(300),
            ))
            .with_watch_warning(AssetSource::get_default_watch_warning())
    }
}

/// A [`Resource`] that hold (repeatable) functions capable of producing new [`AssetReader`](crate::io::AssetReader) and [`AssetWriter`](crate::io::AssetWriter) instances
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

    /// Returns whether there is currently a builder for the given `id`.
    pub fn contains<'b>(&self, id: impl Into<AssetSourceId<'b>>) -> bool {
        match id.into() {
            AssetSourceId::Default => self.default.is_some(),
            AssetSourceId::Name(name) => self.sources.contains_key(&name),
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

    /// Builds a new [`AssetSources`] collection.
    ///
    /// If `watch` is true, the sources will watch for changes. If `create_root_for_writer` is true,
    /// the sources are told their writers should create the root directory (if it does not exist).
    pub fn build_sources(&mut self, watch: bool, create_root_for_writer: bool) -> AssetSources {
        let mut sources = <HashMap<_, _>>::default();
        for (id, source) in &mut self.sources {
            let source = source.build(
                AssetSourceId::Name(id.clone_owned()),
                watch,
                create_root_for_writer,
            );
            sources.insert(id.clone_owned(), source);
        }

        AssetSources {
            sources,
            default: self
                .default
                .as_mut()
                .map(|p| p.build(AssetSourceId::Default, watch, create_root_for_writer))
                .expect(MISSING_DEFAULT_SOURCE),
        }
    }

    /// Builds these sources to be used as unprocessed sources which we intend to process.
    pub(crate) fn build_unprocessed_sources(
        &mut self,
    ) -> HashMap<AssetSourceId<'static>, AssetSource> {
        // Unprocessed sources are only built for processing them, so we hard-code watching their
        // assets to true.
        const WATCH: bool = true;
        // We don't intend to write to the unprocessed sources, so we can avoid create the root
        // directory for it.
        const CREATE_ROOT_FOR_WRITER: bool = false;

        let mut sources = HashMap::default();
        for (id, source) in &mut self.sources {
            let source = source.build(
                AssetSourceId::Name(id.clone_owned()),
                WATCH,
                CREATE_ROOT_FOR_WRITER,
            );
            sources.insert(AssetSourceId::Name(id.clone_owned()), source);
        }

        if let Some(default) = self.default.as_mut() {
            sources.insert(
                AssetSourceId::Default,
                default.build(AssetSourceId::Default, WATCH, CREATE_ROOT_FOR_WRITER),
            );
        }

        sources
    }

    /// Initializes the default [`AssetSourceBuilder`] if it has not already been set.
    pub fn init_default_source(&mut self, path: &str) {
        self.default
            .get_or_insert_with(|| AssetSourceBuilder::platform_default(path));
    }

    pub(crate) fn ids<'a>(&'a self) -> impl Iterator<Item = AssetSourceId<'a>> {
        self.default
            .is_some()
            .then_some(AssetSourceId::Default)
            .into_iter()
            .chain(self.sources.keys().cloned().map(AssetSourceId::Name))
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct UnprocessedAssetSourceBuilders(pub(crate) AssetSourceBuilders);

/// A collection of unprocessed and processed [`AssetReader`](crate::io::AssetReader), [`AssetWriter`](crate::io::AssetWriter), and [`AssetWatcher`] instances
/// for a specific asset source, identified by an [`AssetSourceId`].
pub struct AssetSource {
    id: AssetSourceId<'static>,
    reader: Arc<dyn ErasedAssetReader>,
    writer: Option<Box<dyn ErasedAssetWriter>>,
    watcher: Option<Box<dyn AssetWatcher>>,
    event_receiver: Option<async_channel::Receiver<AssetSourceEvent>>,
}

impl AssetSource {
    /// Returns this source's id.
    #[inline]
    pub fn id(&self) -> AssetSourceId<'static> {
        self.id.clone()
    }

    /// Return's this source's unprocessed [`AssetReader`](crate::io::AssetReader).
    #[inline]
    pub fn reader(&self) -> &dyn ErasedAssetReader {
        &*self.reader
    }

    /// Return's this source's unprocessed [`AssetWriter`](crate::io::AssetWriter), if it exists.
    #[inline]
    pub fn writer(&self) -> Result<&dyn ErasedAssetWriter, MissingAssetWriterError> {
        self.writer
            .as_deref()
            .ok_or_else(|| MissingAssetWriterError(self.id.clone_owned()))
    }

    /// Return's this source's unprocessed event receiver, if the source is currently watching for changes.
    #[inline]
    pub fn event_receiver(&self) -> Option<&async_channel::Receiver<AssetSourceEvent>> {
        self.event_receiver.as_ref()
    }

    /// Returns a builder function for this platform's default [`AssetReader`](crate::io::AssetReader). `path` is the relative path to
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

    /// Returns a builder function for this platform's default [`AssetWriter`](crate::io::AssetWriter). `path` is the relative path to
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
    #[cfg_attr(
        any(
            not(feature = "file_watcher"),
            target_arch = "wasm32",
            target_os = "android"
        ),
        expect(
            unused_variables,
            reason = "The `path` and `file_debounce_wait_time` arguments are unused when on WASM, Android, or if the `file_watcher` feature is disabled."
        )
    )]
    pub fn get_default_watcher(
        path: String,
        file_debounce_wait_time: Duration,
    ) -> impl FnMut(async_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>> + Send + Sync
    {
        move |sender: async_channel::Sender<AssetSourceEvent>| {
            #[cfg(all(
                feature = "file_watcher",
                not(target_arch = "wasm32"),
                not(target_os = "android")
            ))]
            {
                let path = super::file::get_base_path().join(path.clone());
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

    /// Wraps the [`AssetReader`] so that [`AssetReader`] futures (such as [`AssetReader::read`] to
    /// wait until the [`AssetProcessor`] has finished processing the requested asset.
    ///
    /// Returns the ungated reader to allow the [`AssetProcessor`] to read without blocking itself,
    /// and the writer, as only the processor is allowed to write to a processed asset source.
    ///
    /// [`AssetReader`]: crate::io::AssetReader
    /// [`AssetReader::read`]: crate::io::AssetReader::read
    /// [`AssetProcessor`]: crate::AssetProcessor
    pub(crate) fn gate_on_processor(
        &mut self,
        processing_state: Arc<ProcessingState>,
    ) -> (Arc<dyn ErasedAssetReader>, Box<dyn ErasedAssetWriter>) {
        let reader = self.reader.clone();
        self.reader = Arc::new(ProcessorGatedReader::new(
            self.id(),
            reader.clone(),
            processing_state,
        ));
        let writer = self
            .writer
            .take()
            .expect("processed asset sources must include a writer");
        (reader, writer)
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
                .ok_or(MissingAssetSourceError(AssetSourceId::Name(name))),
        }
    }

    /// Gets the [`AssetSource`] mutably with the given `id`, if it exists.
    fn get_mut<'a, 'b>(
        &'a mut self,
        id: impl Into<AssetSourceId<'b>>,
    ) -> Result<&'a mut AssetSource, MissingAssetSourceError> {
        match id.into().into_owned() {
            AssetSourceId::Default => Ok(&mut self.default),
            AssetSourceId::Name(name) => self
                .sources
                .get_mut(&name)
                .ok_or(MissingAssetSourceError(AssetSourceId::Name(name))),
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

    /// Iterates over the [`AssetSourceId`] of every [`AssetSource`] in the collection (including the default source).
    pub fn ids(&self) -> impl Iterator<Item = AssetSourceId<'static>> + '_ {
        self.sources
            .keys()
            .map(|k| AssetSourceId::Name(k.clone_owned()))
            .chain(Some(AssetSourceId::Default))
    }

    /// Wraps the [`AssetReader`] of every source in `self` with a corresponding entry in
    /// `unprocessed_sources`, so that [`AssetReader`] futures (such as [`AssetReader::read`] wait
    /// until the [`AssetProcessor`] has finished processing the requested asset.
    ///
    /// Panics if there is a source in `unprocessed_sources` without a corresponding source in
    /// `self`.
    ///
    /// Returns the ungated reader and the writer for each processed source.
    ///
    /// [`AssetReader`]: crate::io::AssetReader
    /// [`AssetReader::read`]: crate::io::AssetReader::read
    /// [`AssetProcessor`]: crate::AssetProcessor
    pub(crate) fn gate_on_processor(
        &mut self,
        unprocessed_sources: &HashMap<AssetSourceId<'static>, AssetSource>,
        processing_state: Arc<ProcessingState>,
    ) -> HashMap<AssetSourceId<'static>, (Arc<dyn ErasedAssetReader>, Box<dyn ErasedAssetWriter>)>
    {
        let mut source_id_to_ungated_reader_and_writer = HashMap::new();
        for (id, _) in unprocessed_sources.iter() {
            let ungated_reader_and_writer = self
                .get_mut(id)
                .expect("every unprocessed source should have a corresponding final source")
                .gate_on_processor(processing_state.clone());
            source_id_to_ungated_reader_and_writer.insert(id.clone(), ungated_reader_and_writer);
        }
        source_id_to_ungated_reader_and_writer
    }
}

/// An error returned when an [`AssetSource`] does not exist for a given id.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Source '{0}' does not exist")]
pub struct MissingAssetSourceError(pub(crate) AssetSourceId<'static>);

/// An error returned when an [`AssetWriter`](crate::io::AssetWriter) does not exist for a given id.
#[derive(Error, Debug, Clone)]
#[error("Asset Source '{0}' does not have an AssetWriter.")]
pub struct MissingAssetWriterError(AssetSourceId<'static>);

/// An error returned when a processed [`AssetReader`](crate::io::AssetReader) does not exist for a given id.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Source '{0}' does not have a processed AssetReader.")]
pub struct MissingProcessedAssetReaderError(AssetSourceId<'static>);

/// An error returned when a processed [`AssetWriter`](crate::io::AssetWriter) does not exist for a given id.
#[derive(Error, Debug, Clone)]
#[error("Asset Source '{0}' does not have a processed AssetWriter.")]
pub struct MissingProcessedAssetWriterError(AssetSourceId<'static>);

const MISSING_DEFAULT_SOURCE: &str =
    "A default AssetSource is required. Add one to `AssetSourceBuilders`";
