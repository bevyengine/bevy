use crate::{
    io::{
        processor_gated::ProcessorGatedReader, AssetProviderEvent, AssetReader, AssetWatcher,
        AssetWriter,
    },
    processor::AssetProcessorData,
};
use bevy_ecs::system::Resource;
use bevy_log::{error, warn};
use bevy_utils::{CowArc, Duration, HashMap};
use std::{fmt::Display, hash::Hash, sync::Arc};
use thiserror::Error;

/// A reference to an "asset provider", which maps to an [`AssetReader`] and/or [`AssetWriter`].
///
/// * [`AssetProviderId::Default`] corresponds to "default asset paths" that don't specify a provider: `/path/to/asset.png`
/// * [`AssetProviderId::Name`] corresponds to asset paths that _do_ specify a provider: `remote://path/to/asset.png`, where `remote` is the name.
#[derive(Default, Clone, Debug, Eq)]
pub enum AssetProviderId<'a> {
    /// The default asset provider.
    #[default]
    Default,
    /// A non-default named asset provider.
    Name(CowArc<'a, str>),
}

impl<'a> Display for AssetProviderId<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_str() {
            None => write!(f, "AssetProviderId::Default"),
            Some(v) => write!(f, "AssetProviderId::Name({v})"),
        }
    }
}

impl<'a> AssetProviderId<'a> {
    /// Creates a new [`AssetProviderId`]
    pub fn new(provider: Option<impl Into<CowArc<'a, str>>>) -> AssetProviderId<'a> {
        match provider {
            Some(provider) => AssetProviderId::Name(provider.into()),
            None => AssetProviderId::Default,
        }
    }

    /// Returns [`None`] if this is [`AssetProviderId::Default`] and [`Some`] containing the
    /// the name if this is [`AssetProviderId::Name`].  
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AssetProviderId::Default => None,
            AssetProviderId::Name(v) => Some(v),
        }
    }

    /// If this is not already an owned / static id, create one. Otherwise, it will return itself (with a static lifetime).
    pub fn into_owned(self) -> AssetProviderId<'static> {
        match self {
            AssetProviderId::Default => AssetProviderId::Default,
            AssetProviderId::Name(v) => AssetProviderId::Name(v.into_owned()),
        }
    }

    /// Clones into an owned [`AssetProviderId<'static>`].
    /// This is equivalent to `.clone().into_owned()`.
    #[inline]
    pub fn clone_owned(&self) -> AssetProviderId<'static> {
        self.clone().into_owned()
    }
}

impl From<&'static str> for AssetProviderId<'static> {
    fn from(value: &'static str) -> Self {
        AssetProviderId::Name(value.into())
    }
}

impl<'a, 'b> From<&'a AssetProviderId<'b>> for AssetProviderId<'b> {
    fn from(value: &'a AssetProviderId<'b>) -> Self {
        value.clone()
    }
}

impl From<Option<&'static str>> for AssetProviderId<'static> {
    fn from(value: Option<&'static str>) -> Self {
        match value {
            Some(value) => AssetProviderId::Name(value.into()),
            None => AssetProviderId::Default,
        }
    }
}

impl From<String> for AssetProviderId<'static> {
    fn from(value: String) -> Self {
        AssetProviderId::Name(value.into())
    }
}

impl<'a> Hash for AssetProviderId<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl<'a> PartialEq for AssetProviderId<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(&other.as_str())
    }
}

/// Metadata about an "asset provider", such as how to construct the [`AssetReader`] and [`AssetWriter`] for the provider,
/// and whether or not the provider is processed.
#[derive(Default)]
pub struct AssetProviderBuilder {
    pub reader: Option<Box<dyn FnMut() -> Box<dyn AssetReader> + Send + Sync>>,
    pub writer: Option<Box<dyn FnMut() -> Option<Box<dyn AssetWriter>> + Send + Sync>>,
    pub watcher: Option<
        Box<
            dyn FnMut(
                    crossbeam_channel::Sender<AssetProviderEvent>,
                ) -> Option<Box<dyn AssetWatcher>>
                + Send
                + Sync,
        >,
    >,
    pub processed_reader: Option<Box<dyn FnMut() -> Box<dyn AssetReader> + Send + Sync>>,
    pub processed_writer: Option<Box<dyn FnMut() -> Option<Box<dyn AssetWriter>> + Send + Sync>>,
    pub processed_watcher: Option<
        Box<
            dyn FnMut(
                    crossbeam_channel::Sender<AssetProviderEvent>,
                ) -> Option<Box<dyn AssetWatcher>>
                + Send
                + Sync,
        >,
    >,
}

impl AssetProviderBuilder {
    /// Builds a new [`AssetProvider`] with the given `id`. If `watch` is true, the unprocessed provider will watch for changes.
    /// If `watch_processed` is true, the processed provider will watch for changes.
    pub fn build(
        &mut self,
        id: AssetProviderId<'static>,
        watch: bool,
        watch_processed: bool,
    ) -> Option<AssetProvider> {
        let reader = (self.reader.as_mut()?)();
        let writer = self.writer.as_mut().map(|w| match (w)() {
            Some(w) => w,
            None => panic!("{} does not have an AssetWriter configured. Note that Web and Android do not currently support writing assets.", id),
        });
        let processed_writer = self.processed_writer.as_mut().map(|w| match (w)() {
            Some(w) => w,
            None => panic!("{} does not have a processed AssetWriter configured. Note that Web and Android do not currently support writing assets.", id),
        });
        let mut provider = AssetProvider {
            id: id.clone(),
            reader,
            writer,
            processed_reader: self.processed_reader.as_mut().map(|r| (r)()),
            processed_writer,
            event_receiver: None,
            watcher: None,
            processed_event_receiver: None,
            processed_watcher: None,
        };

        if watch {
            let (sender, receiver) = crossbeam_channel::unbounded();
            match self.watcher.as_mut().and_then(|w|(w)(sender)) {
                Some(w) => {
                    provider.watcher = Some(w);
                    provider.event_receiver = Some(receiver);
                },
                None => warn!("{id} does not have an AssetWatcher configured. Consider enabling the `file_watcher` feature. Note that Web and Android do not currently support watching assets."),
            }
        }

        if watch_processed {
            let (sender, receiver) = crossbeam_channel::unbounded();
            match self.processed_watcher.as_mut().and_then(|w|(w)(sender)) {
                Some(w) => {
                    provider.processed_watcher = Some(w);
                    provider.processed_event_receiver = Some(receiver);
                },
                None => warn!("{id} does not have a processed AssetWatcher configured. Consider enabling the `file_watcher` feature. Note that Web and Android do not currently support watching assets."),
            }
        }
        Some(provider)
    }

    /// Will use the given `reader` function to construct unprocessed [`AssetReader`] instances.
    pub fn with_reader(
        mut self,
        reader: impl FnMut() -> Box<dyn AssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.reader = Some(Box::new(reader));
        self
    }

    /// Will use the given `writer` function to construct unprocessed [`AssetWriter`] instances.
    pub fn with_writer(
        mut self,
        writer: impl FnMut() -> Option<Box<dyn AssetWriter>> + Send + Sync + 'static,
    ) -> Self {
        self.writer = Some(Box::new(writer));
        self
    }

    /// Will use the given `watcher` function to construct unprocessed [`AssetWatcher`] instances.
    pub fn with_watcher(
        mut self,
        watcher: impl FnMut(crossbeam_channel::Sender<AssetProviderEvent>) -> Option<Box<dyn AssetWatcher>>
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
        reader: impl FnMut() -> Box<dyn AssetReader> + Send + Sync + 'static,
    ) -> Self {
        self.processed_reader = Some(Box::new(reader));
        self
    }

    /// Will use the given `writer` function to construct processed [`AssetWriter`] instances.
    pub fn with_processed_writer(
        mut self,
        writer: impl FnMut() -> Option<Box<dyn AssetWriter>> + Send + Sync + 'static,
    ) -> Self {
        self.processed_writer = Some(Box::new(writer));
        self
    }

    /// Will use the given `watcher` function to construct processed [`AssetWatcher`] instances.
    pub fn with_processed_watcher(
        mut self,
        watcher: impl FnMut(crossbeam_channel::Sender<AssetProviderEvent>) -> Option<Box<dyn AssetWatcher>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.processed_watcher = Some(Box::new(watcher));
        self
    }
}

/// A [`Resource`] that hold (repeatable) functions capable of producing new [`AssetReader`] and [`AssetWriter`] instances
/// for a given asset provider.
#[derive(Resource, Default)]
pub struct AssetProviderBuilders {
    providers: HashMap<CowArc<'static, str>, AssetProviderBuilder>,
    default: Option<AssetProviderBuilder>,
}

impl AssetProviderBuilders {
    /// Inserts a new builder with the given `id`
    pub fn insert(
        &mut self,
        id: impl Into<AssetProviderId<'static>>,
        provider: AssetProviderBuilder,
    ) {
        match id.into() {
            AssetProviderId::Default => {
                self.default = Some(provider);
            }
            AssetProviderId::Name(name) => {
                self.providers.insert(name, provider);
            }
        }
    }

    /// Gets a mutable builder with the given `id`, if it exists.
    pub fn get_mut<'a, 'b>(
        &'a mut self,
        id: impl Into<AssetProviderId<'b>>,
    ) -> Option<&'a mut AssetProviderBuilder> {
        match id.into() {
            AssetProviderId::Default => self.default.as_mut(),
            AssetProviderId::Name(name) => self.providers.get_mut(&name.into_owned()),
        }
    }

    /// Builds an new [`AssetProviders`] collection. If `watch` is true, the unprocessed providers will watch for changes.
    /// If `watch_processed` is true, the processed providers will watch for changes.
    pub fn build_providers(&mut self, watch: bool, watch_processed: bool) -> AssetProviders {
        let mut providers = HashMap::new();
        for (id, provider) in &mut self.providers {
            if let Some(data) = provider.build(
                AssetProviderId::Name(id.clone_owned()),
                watch,
                watch_processed,
            ) {
                providers.insert(id.clone_owned(), data);
            }
        }

        AssetProviders {
            providers,
            default: self
                .default
                .as_mut()
                .and_then(|p| p.build(AssetProviderId::Default, watch, watch_processed))
                .expect(MISSING_DEFAULT_PROVIDER),
        }
    }

    /// Initializes the default [`AssetProviderBuilder`] if it has not already been set.
    pub fn init_default_providers(&mut self, path: &str, processed_path: &str) {
        self.default.get_or_insert_with(|| {
            AssetProviderBuilder::default()
                .with_reader(AssetProvider::get_default_reader(path.to_string()))
                .with_writer(AssetProvider::get_default_writer(path.to_string()))
                .with_watcher(AssetProvider::get_default_watcher(
                    path.to_string(),
                    Duration::from_millis(300),
                ))
                .with_processed_reader(AssetProvider::get_default_reader(
                    processed_path.to_string(),
                ))
                .with_processed_writer(AssetProvider::get_default_writer(
                    processed_path.to_string(),
                ))
                .with_processed_watcher(AssetProvider::get_default_watcher(
                    processed_path.to_string(),
                    Duration::from_millis(300),
                ))
        });
    }
}

/// A collection of unprocessed and processed [`AssetReader`], [`AssetWriter`], and [`AssetWatcher`] instances
/// for a specific asset provider, identified by an [`AssetProviderId`].
pub struct AssetProvider {
    id: AssetProviderId<'static>,
    reader: Box<dyn AssetReader>,
    writer: Option<Box<dyn AssetWriter>>,
    processed_reader: Option<Box<dyn AssetReader>>,
    processed_writer: Option<Box<dyn AssetWriter>>,
    watcher: Option<Box<dyn AssetWatcher>>,
    processed_watcher: Option<Box<dyn AssetWatcher>>,
    event_receiver: Option<crossbeam_channel::Receiver<AssetProviderEvent>>,
    processed_event_receiver: Option<crossbeam_channel::Receiver<AssetProviderEvent>>,
}

impl AssetProvider {
    /// Starts building a new [`AssetProvider`].
    pub fn build() -> AssetProviderBuilder {
        AssetProviderBuilder::default()
    }

    /// Returns this provider's id.
    #[inline]
    pub fn id(&self) -> AssetProviderId<'static> {
        self.id.clone()
    }

    /// Return's this provider's unprocessed [`AssetReader`].
    #[inline]
    pub fn reader(&self) -> &dyn AssetReader {
        &*self.reader
    }

    /// Return's this provider's unprocessed [`AssetWriter`], if it exists.
    #[inline]
    pub fn writer(&self) -> Result<&dyn AssetWriter, MissingAssetWriterError> {
        self.writer
            .as_deref()
            .ok_or_else(|| MissingAssetWriterError(self.id.clone_owned()))
    }

    /// Return's this provider's processed [`AssetReader`], if it exists.
    #[inline]
    pub fn processed_reader(&self) -> Result<&dyn AssetReader, MissingProcessedAssetReaderError> {
        self.processed_reader
            .as_deref()
            .ok_or_else(|| MissingProcessedAssetReaderError(self.id.clone_owned()))
    }

    /// Return's this provider's processed [`AssetWriter`], if it exists.
    #[inline]
    pub fn processed_writer(&self) -> Result<&dyn AssetWriter, MissingProcessedAssetWriterError> {
        self.processed_writer
            .as_deref()
            .ok_or_else(|| MissingProcessedAssetWriterError(self.id.clone_owned()))
    }

    /// Return's this provider's unprocessed event receiver, if the provider is currently watching for changes.
    #[inline]
    pub fn event_receiver(&self) -> Option<&crossbeam_channel::Receiver<AssetProviderEvent>> {
        self.event_receiver.as_ref()
    }

    /// Return's this provider's processed event receiver, if the provider is currently watching for changes.
    #[inline]
    pub fn processed_event_receiver(
        &self,
    ) -> Option<&crossbeam_channel::Receiver<AssetProviderEvent>> {
        self.processed_event_receiver.as_ref()
    }

    /// Returns true if the assets in this provider should be processed.
    #[inline]
    pub fn should_process(&self) -> bool {
        self.processed_writer.is_some()
    }

    /// Returns a builder function for this platform's default [`AssetReader`]. `path` is the relative path to
    /// the asset root.
    pub fn get_default_reader(path: String) -> impl FnMut() -> Box<dyn AssetReader> + Send + Sync {
        move || {
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            return Box::new(super::file::FileAssetReader::new(&path));
            #[cfg(target_arch = "wasm32")]
            return Box::new(super::wasm::HttpWasmAssetReader::new(&path));
            #[cfg(target_os = "android")]
            return Box::new(super::android::AndroidAssetReader);
        }
    }

    /// Returns a builder function for this platform's default [`AssetWriter`]. `path` is the relative path to
    /// the asset root. This will return [`None`] if this platform does not support writing assets by default.
    pub fn get_default_writer(
        path: String,
    ) -> impl FnMut() -> Option<Box<dyn AssetWriter>> + Send + Sync {
        move || {
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            return Some(Box::new(super::file::FileAssetWriter::new(&path)));
            #[cfg(any(target_arch = "wasm32", target_os = "android"))]
            return None;
        }
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
    ) -> impl FnMut(crossbeam_channel::Sender<AssetProviderEvent>) -> Option<Box<dyn AssetWatcher>>
           + Send
           + Sync {
        move |sender: crossbeam_channel::Sender<AssetProviderEvent>| {
            #[cfg(all(
                feature = "file_watcher",
                not(target_arch = "wasm32"),
                not(target_os = "android")
            ))]
            return Some(Box::new(
                super::file::FileWatcher::new(
                    std::path::PathBuf::from(path.clone()),
                    sender,
                    file_debounce_wait_time,
                )
                .unwrap(),
            ));
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

/// A collection of [`AssetProviders`].
pub struct AssetProviders {
    providers: HashMap<CowArc<'static, str>, AssetProvider>,
    default: AssetProvider,
}

impl AssetProviders {
    /// Gets the [`AssetProvider`] with the given `id`, if it exists.
    pub fn get<'a, 'b>(
        &'a self,
        id: impl Into<AssetProviderId<'b>>,
    ) -> Result<&'a AssetProvider, MissingAssetProviderError> {
        match id.into().into_owned() {
            AssetProviderId::Default => Ok(&self.default),
            AssetProviderId::Name(name) => self
                .providers
                .get(&name)
                .ok_or_else(|| MissingAssetProviderError(AssetProviderId::Name(name))),
        }
    }

    /// Iterates all asset providers in the collection (including the default provider).
    pub fn iter(&self) -> impl Iterator<Item = &AssetProvider> {
        self.providers.values().chain(Some(&self.default))
    }

    /// Mutably iterates all asset providers in the collection (including the default provider).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut AssetProvider> {
        self.providers.values_mut().chain(Some(&mut self.default))
    }

    /// Iterates all processed asset providers in the collection (including the default provider).
    pub fn iter_processed(&self) -> impl Iterator<Item = &AssetProvider> {
        self.iter().filter(|p| p.should_process())
    }

    /// Mutably iterates all processed asset providers in the collection (including the default provider).
    pub fn iter_processed_mut(&mut self) -> impl Iterator<Item = &mut AssetProvider> {
        self.iter_mut().filter(|p| p.should_process())
    }

    /// Iterates over the [`AssetProviderId`] of every [`AssetProvider`] in the collection (including the default provider).
    pub fn provider_ids(&self) -> impl Iterator<Item = AssetProviderId<'static>> + '_ {
        self.providers
            .keys()
            .map(|k| AssetProviderId::Name(k.clone_owned()))
            .chain(Some(AssetProviderId::Default))
    }

    /// This will cause processed [`AssetReader`] futures (such as [`AssetReader::read`]) to wait until
    /// the [`AssetProcessor`](crate::AssetProcessor) has finished processing the requested asset.
    pub fn gate_on_processor(&mut self, processor_data: Arc<AssetProcessorData>) {
        for provider in self.iter_processed_mut() {
            provider.gate_on_processor(processor_data.clone());
        }
    }
}

/// An error returned when an [`AssetProvider`] does not exist for a given id.
#[derive(Error, Debug)]
#[error("Asset Provider '{0}' does not exist")]
pub struct MissingAssetProviderError(AssetProviderId<'static>);

/// An error returned when an [`AssetWriter`] does not exist for a given id.
#[derive(Error, Debug)]
#[error("Asset Provider '{0}' does not have an AssetWriter.")]
pub struct MissingAssetWriterError(AssetProviderId<'static>);

/// An error returned when a processed [`AssetReader`] does not exist for a given id.
#[derive(Error, Debug)]
#[error("Asset Provider '{0}' does not have a processed AssetReader.")]
pub struct MissingProcessedAssetReaderError(AssetProviderId<'static>);

/// An error returned when a processed [`AssetWriter`] does not exist for a given id.
#[derive(Error, Debug)]
#[error("Asset Provider '{0}' does not have a processed AssetWriter.")]
pub struct MissingProcessedAssetWriterError(AssetProviderId<'static>);

const MISSING_DEFAULT_PROVIDER: &str =
    "A default AssetProvider is required. Add one to `AssetProviderBuilders`";
