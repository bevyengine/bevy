use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use async_lock::{RwLock, RwLockWriteGuard};
use bevy_platform::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};
use bevy_reflect::TypePath;
use core::marker::PhantomData;
use futures_lite::AsyncWriteExt;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

use bevy_app::{App, TaskPoolPlugin};
use bevy_ecs::error::BevyError;
use bevy_tasks::BoxedFuture;

use crate::{
    io::{
        memory::{Dir, MemoryAssetReader, MemoryAssetWriter},
        AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceBuilders, AssetSourceEvent,
        AssetSourceId, AssetWatcher, PathStream, Reader, ReaderRequiredFeatures,
    },
    processor::{
        AssetProcessor, GetProcessorError, LoadTransformAndSave, LogEntry, Process, ProcessContext,
        ProcessError, ProcessStatus, ProcessorState, ProcessorTransactionLog,
        ProcessorTransactionLogFactory, WriterContext,
    },
    saver::AssetSaver,
    tests::{
        read_asset_as_string, read_meta_as_string, run_app_until, CoolText, CoolTextLoader,
        CoolTextRon, SubText,
    },
    transformer::{AssetTransformer, TransformedAsset},
    Asset, AssetApp, AssetLoader, AssetMode, AssetPath, AssetPlugin, AssetServer, Assets,
    LoadContext, WriteDefaultMetaError,
};

#[derive(TypePath)]
struct MyProcessor<T>(PhantomData<fn() -> T>);

impl<T: TypePath + 'static> Process for MyProcessor<T> {
    type Settings = ();

    async fn process(
        &self,
        _context: &mut ProcessContext<'_>,
        _settings: &Self::Settings,
        _writer: WriterContext<'_>,
    ) -> Result<(), ProcessError> {
        Ok(())
    }
}

#[derive(TypePath)]
struct Marker;

fn create_empty_asset_processor() -> AssetProcessor {
    let mut sources = AssetSourceBuilders::default();
    // Create an empty asset source so that AssetProcessor is happy.
    let dir = Dir::default();
    let memory_reader = MemoryAssetReader { root: dir.clone() };
    sources.insert(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || Box::new(memory_reader.clone())),
    );

    AssetProcessor::new(&mut sources, false).0
}

#[test]
fn get_asset_processor_by_name() {
    let asset_processor = create_empty_asset_processor();
    asset_processor.register_processor(MyProcessor::<Marker>(PhantomData));

    let long_processor = asset_processor
        .get_processor(
            "bevy_asset::processor::tests::MyProcessor<bevy_asset::processor::tests::Marker>",
        )
        .expect("Processor was previously registered");
    let short_processor = asset_processor
        .get_processor("MyProcessor<Marker>")
        .expect("Processor was previously registered");

    // We can use either the long or short processor name and we will get the same processor
    // out.
    assert!(Arc::ptr_eq(&long_processor, &short_processor));
}

#[test]
fn missing_processor_returns_error() {
    let asset_processor = create_empty_asset_processor();

    let Err(long_processor_err) = asset_processor.get_processor(
        "bevy_asset::processor::tests::MyProcessor<bevy_asset::processor::tests::Marker>",
    ) else {
        panic!("Processor was returned even though we never registered any.");
    };
    let GetProcessorError::Missing(long_processor_err) = &long_processor_err else {
        panic!("get_processor returned incorrect error: {long_processor_err}");
    };
    assert_eq!(
        long_processor_err,
        "bevy_asset::processor::tests::MyProcessor<bevy_asset::processor::tests::Marker>"
    );

    // Short paths should also return an error.

    let Err(long_processor_err) = asset_processor.get_processor("MyProcessor<Marker>") else {
        panic!("Processor was returned even though we never registered any.");
    };
    let GetProcessorError::Missing(long_processor_err) = &long_processor_err else {
        panic!("get_processor returned incorrect error: {long_processor_err}");
    };
    assert_eq!(long_processor_err, "MyProcessor<Marker>");
}

// Create another marker type whose short name will overlap `Marker`.
mod sneaky {
    use bevy_reflect::TypePath;

    #[derive(TypePath)]
    pub struct Marker;
}

#[test]
fn ambiguous_short_path_returns_error() {
    let asset_processor = create_empty_asset_processor();
    asset_processor.register_processor(MyProcessor::<Marker>(PhantomData));
    asset_processor.register_processor(MyProcessor::<sneaky::Marker>(PhantomData));

    let Err(long_processor_err) = asset_processor.get_processor("MyProcessor<Marker>") else {
        panic!("Processor was returned even though the short path is ambiguous.");
    };
    let GetProcessorError::Ambiguous {
        processor_short_name,
        ambiguous_processor_names,
    } = &long_processor_err
    else {
        panic!("get_processor returned incorrect error: {long_processor_err}");
    };
    assert_eq!(processor_short_name, "MyProcessor<Marker>");
    let expected_ambiguous_names = [
        "bevy_asset::processor::tests::MyProcessor<bevy_asset::processor::tests::Marker>",
        "bevy_asset::processor::tests::MyProcessor<bevy_asset::processor::tests::sneaky::Marker>",
    ];
    assert_eq!(ambiguous_processor_names, &expected_ambiguous_names);

    let processor_1 = asset_processor
        .get_processor(
            "bevy_asset::processor::tests::MyProcessor<bevy_asset::processor::tests::Marker>",
        )
        .expect("Processor was previously registered");
    let processor_2 = asset_processor
            .get_processor(
                "bevy_asset::processor::tests::MyProcessor<bevy_asset::processor::tests::sneaky::Marker>",
            )
            .expect("Processor was previously registered");

    // If we fully specify the paths, we get the two different processors.
    assert!(!Arc::ptr_eq(&processor_1, &processor_2));
}

#[derive(Clone)]
struct ProcessingDirs {
    source: Dir,
    processed: Dir,
    source_event_sender: async_channel::Sender<AssetSourceEvent>,
}

struct AppWithProcessor {
    app: App,
    source_gate: Arc<RwLock<()>>,
    default_source_dirs: ProcessingDirs,
    extra_sources_dirs: HashMap<String, ProcessingDirs>,
}

/// Similar to [`crate::io::gated::GatedReader`], but uses a lock instead of a channel to avoid
/// needing to send the "correct" number of messages.
#[derive(Clone)]
struct LockGatedReader<R: AssetReader> {
    reader: R,
    gate: Arc<RwLock<()>>,
}

impl<R: AssetReader> LockGatedReader<R> {
    /// Creates a new [`GatedReader`], which wraps the given `reader`. Also returns a [`GateOpener`] which
    /// can be used to open "path gates" for this [`GatedReader`].
    fn new(gate: Arc<RwLock<()>>, reader: R) -> Self {
        Self { gate, reader }
    }
}

impl<R: AssetReader> AssetReader for LockGatedReader<R> {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
        required_features: ReaderRequiredFeatures,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        let _guard = self.gate.read().await;
        self.reader.read(path, required_features).await
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let _guard = self.gate.read().await;
        self.reader.read_meta(path).await
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let _guard = self.gate.read().await;
        self.reader.read_directory(path).await
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let _guard = self.gate.read().await;
        self.reader.is_directory(path).await
    }
}

/// Serializes `text` into a `CoolText` that can be loaded.
///
/// This doesn't support all the features of `CoolText`, so more complex scenarios may require doing
/// this manually.
fn serialize_as_cool_text(text: &str) -> String {
    let cool_text_ron = CoolTextRon {
        text: text.into(),
        dependencies: vec![],
        embedded_dependencies: vec![],
        sub_texts: vec![],
    };
    ron::ser::to_string_pretty(&cool_text_ron, PrettyConfig::new().new_line("\n")).unwrap()
}

/// Sets the transaction log for the app to a fake one to prevent touching the filesystem.
fn set_fake_transaction_log(app: &mut App) {
    /// A dummy transaction log factory that just creates [`FakeTransactionLog`].
    struct FakeTransactionLogFactory;

    impl ProcessorTransactionLogFactory for FakeTransactionLogFactory {
        fn read(&self) -> BoxedFuture<'_, Result<Vec<LogEntry>, BevyError>> {
            Box::pin(async move { Ok(vec![]) })
        }

        fn create_new_log(
            &self,
        ) -> BoxedFuture<'_, Result<Box<dyn ProcessorTransactionLog>, BevyError>> {
            Box::pin(async move { Ok(Box::new(FakeTransactionLog) as _) })
        }
    }

    /// A dummy transaction log that just drops every log.
    // TODO: In the future it's possible for us to have a test of the transaction log, so making
    // this more complex may be necessary.
    struct FakeTransactionLog;

    impl ProcessorTransactionLog for FakeTransactionLog {
        fn begin_processing<'a>(
            &'a mut self,
            _asset: &'a AssetPath<'_>,
        ) -> BoxedFuture<'a, Result<(), BevyError>> {
            Box::pin(async move { Ok(()) })
        }

        fn end_processing<'a>(
            &'a mut self,
            _asset: &'a AssetPath<'_>,
        ) -> BoxedFuture<'a, Result<(), BevyError>> {
            Box::pin(async move { Ok(()) })
        }

        fn unrecoverable(&mut self) -> BoxedFuture<'_, Result<(), BevyError>> {
            Box::pin(async move { Ok(()) })
        }
    }

    app.world()
        .resource::<AssetProcessor>()
        .data()
        .set_log_factory(Box::new(FakeTransactionLogFactory))
        .unwrap();
}

fn create_app_with_asset_processor(extra_sources: &[String]) -> AppWithProcessor {
    let mut app = App::new();
    let source_gate = Arc::new(RwLock::new(()));

    struct UnfinishedProcessingDirs {
        source: Dir,
        processed: Dir,
        // The receiver channel for the source event sender for the unprocessed source.
        source_event_sender_receiver:
            async_channel::Receiver<async_channel::Sender<AssetSourceEvent>>,
    }

    impl UnfinishedProcessingDirs {
        fn finish(self) -> ProcessingDirs {
            ProcessingDirs {
                source: self.source,
                processed: self.processed,
                // The processor listens for events on the source unconditionally, and we enable
                // watching for the processed source, so both of these channels will be filled.
                source_event_sender: self.source_event_sender_receiver.recv_blocking().unwrap(),
            }
        }
    }

    fn create_source(
        app: &mut App,
        source_id: AssetSourceId<'static>,
        source_gate: Arc<RwLock<()>>,
    ) -> UnfinishedProcessingDirs {
        let source_dir = Dir::default();
        let processed_dir = Dir::default();

        let source_memory_reader = LockGatedReader::new(
            source_gate,
            MemoryAssetReader {
                root: source_dir.clone(),
            },
        );
        let source_memory_writer = MemoryAssetWriter {
            root: source_dir.clone(),
        };
        let processed_memory_reader = MemoryAssetReader {
            root: processed_dir.clone(),
        };
        let processed_memory_writer = MemoryAssetWriter {
            root: processed_dir.clone(),
        };

        let (source_event_sender_sender, source_event_sender_receiver) = async_channel::bounded(1);

        struct FakeWatcher;

        impl AssetWatcher for FakeWatcher {}

        app.register_asset_source(
            source_id,
            AssetSourceBuilder::new(move || Box::new(source_memory_reader.clone()))
                .with_writer(move |_| Some(Box::new(source_memory_writer.clone())))
                .with_watcher(move |sender: async_channel::Sender<AssetSourceEvent>| {
                    source_event_sender_sender.send_blocking(sender).unwrap();
                    Some(Box::new(FakeWatcher))
                })
                .with_processed_reader(move || Box::new(processed_memory_reader.clone()))
                .with_processed_writer(move |_| Some(Box::new(processed_memory_writer.clone()))),
        );

        UnfinishedProcessingDirs {
            source: source_dir,
            processed: processed_dir,
            source_event_sender_receiver,
        }
    }

    let default_source_dirs = create_source(&mut app, AssetSourceId::Default, source_gate.clone());

    let extra_sources_dirs = extra_sources
        .iter()
        .map(|source_name| {
            (
                source_name.clone(),
                create_source(
                    &mut app,
                    AssetSourceId::Name(source_name.clone().into()),
                    source_gate.clone(),
                ),
            )
        })
        .collect::<Vec<_>>();

    app.add_plugins((
        TaskPoolPlugin::default(),
        AssetPlugin {
            mode: AssetMode::Processed,
            use_asset_processor_override: Some(true),
            watch_for_changes_override: Some(true),
            ..Default::default()
        },
    ));

    set_fake_transaction_log(&mut app);

    // Now that we've built the app, finish all the processing dirs.

    AppWithProcessor {
        app,
        source_gate,
        default_source_dirs: default_source_dirs.finish(),
        extra_sources_dirs: extra_sources_dirs
            .into_iter()
            .map(|(name, dirs)| (name, dirs.finish()))
            .collect(),
    }
}

fn run_app_until_finished_processing(app: &mut App, guard: RwLockWriteGuard<'_, ()>) {
    let processor = app.world().resource::<AssetProcessor>().clone();
    // We can't just wait for the processor state to be finished since we could have already
    // finished before, but now that something has changed, we may not have restarted processing
    // yet. So wait for processing to start, then finish.
    run_app_until(app, |_| {
        // Before we even consider whether the processor is started, make sure that none of the
        // receivers have anything left in them. This prevents us accidentally, considering the
        // processor as processing before all the events have been processed.
        for source in processor.sources().iter() {
            let Some(recv) = source.event_receiver() else {
                continue;
            };
            if !recv.is_empty() {
                return None;
            }
        }
        let state = bevy_tasks::block_on(processor.get_state());
        (state == ProcessorState::Processing || state == ProcessorState::Initializing).then_some(())
    });
    drop(guard);
    run_app_until(app, |_| {
        (bevy_tasks::block_on(processor.get_state()) == ProcessorState::Finished).then_some(())
    });
}

#[derive(TypePath)]
struct CoolTextSaver;

impl AssetSaver for CoolTextSaver {
    type Asset = CoolText;
    type Settings = ();
    type OutputLoader = CoolTextLoader;
    type Error = std::io::Error;

    async fn save(
        &self,
        writer: &mut crate::io::Writer,
        asset: crate::saver::SavedAsset<'_, Self::Asset>,
        _: &Self::Settings,
    ) -> Result<(), Self::Error> {
        let ron = CoolTextRon {
            text: asset.text.clone(),
            sub_texts: asset
                .iter_labels()
                .map(|label| asset.get_labeled::<SubText, _>(label).unwrap().text.clone())
                .collect(),
            dependencies: asset
                .dependencies
                .iter()
                .map(|handle| handle.path().unwrap().path())
                .map(|path| path.to_str().unwrap().to_string())
                .collect(),
            // NOTE: We can't handle embedded dependencies in any way, since we need to write to
            // another file to do so.
            embedded_dependencies: vec![],
        };
        let ron = ron::ser::to_string_pretty(&ron, PrettyConfig::new().new_line("\n")).unwrap();
        writer.write_all(ron.as_bytes()).await?;
        Ok(())
    }
}

// Note: while we allow any Fn, since closures are unnameable types, creating a processor with a
// closure cannot be used (since we need to include the name of the transformer in the meta
// file).
#[derive(TypePath)]
struct RootAssetTransformer<M: MutateAsset<A>, A: Asset>(M, PhantomData<fn(&mut A)>);

trait MutateAsset<A: Asset>: TypePath + Send + Sync + 'static {
    fn mutate(&self, asset: &mut A);
}

impl<M: MutateAsset<A>, A: Asset> RootAssetTransformer<M, A> {
    fn new(m: M) -> Self {
        Self(m, PhantomData)
    }
}

impl<M: MutateAsset<A>, A: Asset> AssetTransformer for RootAssetTransformer<M, A> {
    type AssetInput = A;
    type AssetOutput = A;
    type Error = std::io::Error;
    type Settings = ();

    async fn transform<'a>(
        &'a self,
        mut asset: TransformedAsset<A>,
        _settings: &'a Self::Settings,
    ) -> Result<TransformedAsset<A>, Self::Error> {
        self.0.mutate(asset.get_mut());
        Ok(asset)
    }
}

#[derive(TypePath)]
struct AddText(String);

impl MutateAsset<CoolText> for AddText {
    fn mutate(&self, text: &mut CoolText) {
        text.text.push_str(&self.0);
    }
}

#[test]
fn no_meta_or_default_processor_copies_asset() {
    // Assets without a meta file or a default processor should still be accessible in the
    // processed path. Note: This isn't exactly the desired property - we don't want the assets
    // to be copied to the processed directory. We just want these assets to still be loadable
    // if we no longer have the source directory. This could be done with a symlink instead of a
    // copy.

    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    let guard = source_gate.write_blocking();

    let path = Path::new("abc.cool.ron");
    let source_asset = r#"(
    text: "abc",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

    source_dir.insert_asset_text(path, source_asset);

    run_app_until_finished_processing(&mut app, guard);

    let processed_asset = processed_dir.get_asset(path).unwrap();
    let processed_asset = str::from_utf8(processed_asset.value()).unwrap();
    assert_eq!(processed_asset, source_asset);
}

#[test]
fn asset_processor_transforms_asset_default_processor() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;
    app.register_asset_loader(CoolTextLoader)
        .register_asset_processor(CoolTextProcessor::new(
            RootAssetTransformer::new(AddText("_def".into())),
            CoolTextSaver,
        ))
        .set_default_asset_processor::<CoolTextProcessor>("cool.ron");

    let guard = source_gate.write_blocking();

    let path = Path::new("abc.cool.ron");
    source_dir.insert_asset_text(
        path,
        r#"(
    text: "abc",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#,
    );

    run_app_until_finished_processing(&mut app, guard);

    let processed_asset = processed_dir.get_asset(path).unwrap();
    let processed_asset = str::from_utf8(processed_asset.value()).unwrap();
    assert_eq!(
        processed_asset,
        r#"(
    text: "abc_def",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#
    );
}

#[test]
fn asset_processor_transforms_asset_with_meta() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;
    app.register_asset_loader(CoolTextLoader)
        .register_asset_processor(CoolTextProcessor::new(
            RootAssetTransformer::new(AddText("_def".into())),
            CoolTextSaver,
        ));

    let guard = source_gate.write_blocking();

    let path = Path::new("abc.cool.ron");
    source_dir.insert_asset_text(
        path,
        r#"(
    text: "abc",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#,
    );
    source_dir.insert_meta_text(path, r#"(
    meta_format_version: "1.0",
    asset: Process(
        processor: "bevy_asset::processor::process::LoadTransformAndSave<bevy_asset::tests::CoolTextLoader, bevy_asset::processor::tests::RootAssetTransformer<bevy_asset::processor::tests::AddText, bevy_asset::tests::CoolText>, bevy_asset::processor::tests::CoolTextSaver>",
        settings: (
            loader_settings: (),
            transformer_settings: (),
            saver_settings: (),
        ),
    ),
)"#);

    run_app_until_finished_processing(&mut app, guard);

    let processed_asset = processed_dir.get_asset(path).unwrap();
    let processed_asset = str::from_utf8(processed_asset.value()).unwrap();
    assert_eq!(
        processed_asset,
        r#"(
    text: "abc_def",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#
    );
}

#[test]
fn asset_processor_transforms_asset_with_short_path_meta() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;
    app.register_asset_loader(CoolTextLoader)
        .register_asset_processor(CoolTextProcessor::new(
            RootAssetTransformer::new(AddText("_def".into())),
            CoolTextSaver,
        ));

    let guard = source_gate.write_blocking();

    let path = Path::new("abc.cool.ron");
    source_dir.insert_asset_text(
        path,
        r#"(
    text: "abc",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#,
    );
    source_dir.insert_meta_text(path, r#"(
    meta_format_version: "1.0",
    asset: Process(
        processor: "LoadTransformAndSave<CoolTextLoader, RootAssetTransformer<AddText, CoolText>, CoolTextSaver>",
        settings: (
            loader_settings: (),
            transformer_settings: (),
            saver_settings: (),
        ),
    ),
)"#);

    run_app_until_finished_processing(&mut app, guard);

    let processed_asset = processed_dir.get_asset(path).unwrap();
    let processed_asset = str::from_utf8(processed_asset.value()).unwrap();
    assert_eq!(
        processed_asset,
        r#"(
    text: "abc_def",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#
    );
}

#[derive(Asset, TypePath, Serialize, Deserialize)]
struct FakeGltf {
    gltf_nodes: BTreeMap<String, String>,
    gltf_meshes: Vec<String>,
}

#[derive(TypePath)]
struct FakeGltfLoader;

impl AssetLoader for FakeGltfLoader {
    type Asset = FakeGltf;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        use std::io::{Error, ErrorKind};

        let mut bytes = vec![];
        reader.read_to_end(&mut bytes).await?;
        ron::de::from_bytes(&bytes)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))
    }

    fn extensions(&self) -> &[&str] {
        &["gltf"]
    }
}

#[derive(Asset, TypePath, Serialize, Deserialize)]
struct FakeBsn {
    parent_bsn: Option<String>,
    nodes: BTreeMap<String, String>,
}

// This loader loads the BSN but as an "inlined" scene. We read the original BSN and create a
// scene that holds all the data including parents.
// TODO: It would be nice if the inlining was actually done as an `AssetTransformer`, but
// `Process` currently has no way to load nested assets.
#[derive(TypePath)]
struct FakeBsnLoader;

impl AssetLoader for FakeBsnLoader {
    type Asset = FakeBsn;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        use std::io::{Error, ErrorKind};

        let mut bytes = vec![];
        reader.read_to_end(&mut bytes).await?;
        let bsn: FakeBsn = ron::de::from_bytes(&bytes)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;

        if bsn.parent_bsn.is_none() {
            return Ok(bsn);
        }

        let parent_bsn = bsn.parent_bsn.unwrap();
        let parent_bsn = load_context
            .loader()
            .immediate()
            .load(parent_bsn)
            .await
            .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
        let mut new_bsn: FakeBsn = parent_bsn.take();
        for (name, node) in bsn.nodes {
            new_bsn.nodes.insert(name, node);
        }
        Ok(new_bsn)
    }

    fn extensions(&self) -> &[&str] {
        &["bsn"]
    }
}

#[derive(TypePath)]
struct GltfToBsn;

impl AssetTransformer for GltfToBsn {
    type AssetInput = FakeGltf;
    type AssetOutput = FakeBsn;
    type Settings = ();
    type Error = std::io::Error;

    async fn transform<'a>(
        &'a self,
        mut asset: TransformedAsset<Self::AssetInput>,
        _settings: &'a Self::Settings,
    ) -> Result<TransformedAsset<Self::AssetOutput>, Self::Error> {
        let bsn = FakeBsn {
            parent_bsn: None,
            // Pretend we converted all the glTF nodes into BSN's format.
            nodes: core::mem::take(&mut asset.get_mut().gltf_nodes),
        };
        Ok(asset.replace_asset(bsn))
    }
}

#[derive(TypePath)]
struct FakeBsnSaver;

impl AssetSaver for FakeBsnSaver {
    type Asset = FakeBsn;
    type Error = std::io::Error;
    type OutputLoader = FakeBsnLoader;
    type Settings = ();

    async fn save(
        &self,
        writer: &mut crate::io::Writer,
        asset: crate::saver::SavedAsset<'_, Self::Asset>,
        _settings: &Self::Settings,
    ) -> Result<(), Self::Error> {
        use std::io::{Error, ErrorKind};

        let ron_string =
            ron::ser::to_string_pretty(asset.get(), PrettyConfig::new().new_line("\n"))
                .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

        writer.write_all(ron_string.as_bytes()).await
    }
}
#[test]
fn asset_processor_loading_can_read_processed_assets() {
    use crate::transformer::IdentityAssetTransformer;

    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    // This processor loads a gltf file, converts it to BSN and then saves out the BSN.
    type GltfProcessor = LoadTransformAndSave<FakeGltfLoader, GltfToBsn, FakeBsnSaver>;
    // This processor loads a BSN file (which "inlines" parent BSNs at load), and then saves the
    // inlined BSN.
    type BsnProcessor =
        LoadTransformAndSave<FakeBsnLoader, IdentityAssetTransformer<FakeBsn>, FakeBsnSaver>;
    app.register_asset_loader(FakeBsnLoader)
        .register_asset_loader(FakeGltfLoader)
        .register_asset_processor(GltfProcessor::new(GltfToBsn, FakeBsnSaver))
        .register_asset_processor(BsnProcessor::new(
            IdentityAssetTransformer::new(),
            FakeBsnSaver,
        ))
        .set_default_asset_processor::<GltfProcessor>("gltf")
        .set_default_asset_processor::<BsnProcessor>("bsn");

    let guard = source_gate.write_blocking();

    let gltf_path = Path::new("abc.gltf");
    source_dir.insert_asset_text(
        gltf_path,
        r#"(
    gltf_nodes: {
        "name": "thing",
        "position": "123",
    },
    gltf_meshes: [],
)"#,
    );
    let bsn_path = Path::new("def.bsn");
    // The bsn tries to load the gltf as a bsn. This only works if the bsn can read processed
    // assets.
    source_dir.insert_asset_text(
        bsn_path,
        r#"(
    parent_bsn: Some("abc.gltf"),
    nodes: {
        "position": "456",
        "color": "red",
    },
)"#,
    );

    run_app_until_finished_processing(&mut app, guard);

    let processed_bsn = processed_dir.get_asset(bsn_path).unwrap();
    let processed_bsn = str::from_utf8(processed_bsn.value()).unwrap();
    // The processed bsn should have been "inlined", so no parent and "overlaid" nodes.
    assert_eq!(
        processed_bsn,
        r#"(
    parent_bsn: None,
    nodes: {
        "color": "red",
        "name": "thing",
        "position": "456",
    },
)"#
    );
}

#[test]
fn asset_processor_loading_can_read_source_assets() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    #[derive(Serialize, Deserialize)]
    struct FakeGltfxData {
        // These are the file paths to the gltfs.
        gltfs: Vec<String>,
    }

    #[derive(Asset, TypePath)]
    struct FakeGltfx {
        gltfs: Vec<FakeGltf>,
    }

    #[derive(TypePath)]
    struct FakeGltfxLoader;

    impl AssetLoader for FakeGltfxLoader {
        type Asset = FakeGltfx;
        type Error = std::io::Error;
        type Settings = ();

        async fn load(
            &self,
            reader: &mut dyn Reader,
            _settings: &Self::Settings,
            load_context: &mut LoadContext<'_>,
        ) -> Result<Self::Asset, Self::Error> {
            use std::io::{Error, ErrorKind};

            let mut buf = vec![];
            reader.read_to_end(&mut buf).await?;

            let gltfx_data: FakeGltfxData =
                ron::de::from_bytes(&buf).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

            let mut gltfs = vec![];
            for gltf in gltfx_data.gltfs.into_iter() {
                // gltfx files come from "generic" software that doesn't know anything about
                // Bevy, so it needs to load the source assets to make sense.
                let gltf = load_context
                    .loader()
                    .immediate()
                    .load(gltf)
                    .await
                    .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
                gltfs.push(gltf.take());
            }

            Ok(FakeGltfx { gltfs })
        }

        fn extensions(&self) -> &[&str] {
            &["gltfx"]
        }
    }

    #[derive(TypePath)]
    struct GltfxToBsn;

    impl AssetTransformer for GltfxToBsn {
        type AssetInput = FakeGltfx;
        type AssetOutput = FakeBsn;
        type Settings = ();
        type Error = std::io::Error;

        async fn transform<'a>(
            &'a self,
            mut asset: TransformedAsset<Self::AssetInput>,
            _settings: &'a Self::Settings,
        ) -> Result<TransformedAsset<Self::AssetOutput>, Self::Error> {
            let gltfx = asset.get_mut();

            // Merge together all the gltfs from the gltfx into one big bsn.
            let bsn = gltfx.gltfs.drain(..).fold(
                FakeBsn {
                    parent_bsn: None,
                    nodes: Default::default(),
                },
                |mut bsn, gltf| {
                    for (key, value) in gltf.gltf_nodes {
                        bsn.nodes.insert(key, value);
                    }
                    bsn
                },
            );

            Ok(asset.replace_asset(bsn))
        }
    }

    // This processor loads a gltf file, converts it to BSN and then saves out the BSN.
    type GltfProcessor = LoadTransformAndSave<FakeGltfLoader, GltfToBsn, FakeBsnSaver>;
    // This processor loads a gltfx file (including its gltf files) and converts it to BSN.
    type GltfxProcessor = LoadTransformAndSave<FakeGltfxLoader, GltfxToBsn, FakeBsnSaver>;
    app.register_asset_loader(FakeGltfLoader)
        .register_asset_loader(FakeGltfxLoader)
        .register_asset_loader(FakeBsnLoader)
        .register_asset_processor(GltfProcessor::new(GltfToBsn, FakeBsnSaver))
        .register_asset_processor(GltfxProcessor::new(GltfxToBsn, FakeBsnSaver))
        .set_default_asset_processor::<GltfProcessor>("gltf")
        .set_default_asset_processor::<GltfxProcessor>("gltfx");

    let guard = source_gate.write_blocking();

    let gltf_path_1 = Path::new("abc.gltf");
    source_dir.insert_asset_text(
        gltf_path_1,
        r#"(
    gltf_nodes: {
        "name": "thing",
        "position": "123",
    },
    gltf_meshes: [],
)"#,
    );
    let gltf_path_2 = Path::new("def.gltf");
    source_dir.insert_asset_text(
        gltf_path_2,
        r#"(
    gltf_nodes: {
        "velocity": "456",
        "color": "red",
    },
    gltf_meshes: [],
)"#,
    );

    let gltfx_path = Path::new("xyz.gltfx");
    source_dir.insert_asset_text(
        gltfx_path,
        r#"(
    gltfs: ["abc.gltf", "def.gltf"],
)"#,
    );

    run_app_until_finished_processing(&mut app, guard);

    // Sanity check that the two gltf files were actually processed.
    let processed_gltf_1 = processed_dir.get_asset(gltf_path_1).unwrap();
    let processed_gltf_1 = str::from_utf8(processed_gltf_1.value()).unwrap();
    assert_eq!(
        processed_gltf_1,
        r#"(
    parent_bsn: None,
    nodes: {
        "name": "thing",
        "position": "123",
    },
)"#
    );
    let processed_gltf_2 = processed_dir.get_asset(gltf_path_2).unwrap();
    let processed_gltf_2 = str::from_utf8(processed_gltf_2.value()).unwrap();
    assert_eq!(
        processed_gltf_2,
        r#"(
    parent_bsn: None,
    nodes: {
        "color": "red",
        "velocity": "456",
    },
)"#
    );

    // The processed gltfx should have been able to load and merge the gltfs despite them having
    // been processed into bsn.

    // Blocked on https://github.com/bevyengine/bevy/issues/21269. This is the actual assertion.
    //         let processed_gltfx = processed_dir.get_asset(gltfx_path).unwrap();
    //         let processed_gltfx = str::from_utf8(processed_gltfx.value()).unwrap();
    //         assert_eq!(
    //             processed_gltfx,
    //             r#"(
    //     parent_bsn: None,
    //     nodes: {
    //         "color": "red",
    //         "name": "thing",
    //         "position": "123",
    //         "velocity": "456",
    //     },
    // )"#
    //         );

    // This assertion exists to "prove" that this problem exists.
    assert!(processed_dir.get_asset(gltfx_path).is_none());
}

#[test]
fn asset_processor_processes_all_sources() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: default_source_dir,
                processed: default_processed_dir,
                source_event_sender: default_source_events,
            },
        extra_sources_dirs,
    } = create_app_with_asset_processor(&["custom_1".into(), "custom_2".into()]);
    let ProcessingDirs {
        source: custom_1_source_dir,
        processed: custom_1_processed_dir,
        source_event_sender: custom_1_source_events,
    } = extra_sources_dirs["custom_1"].clone();
    let ProcessingDirs {
        source: custom_2_source_dir,
        processed: custom_2_processed_dir,
        source_event_sender: custom_2_source_events,
    } = extra_sources_dirs["custom_2"].clone();

    type AddTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;
    app.init_asset::<CoolText>()
        .init_asset::<SubText>()
        .register_asset_loader(CoolTextLoader)
        .register_asset_processor(AddTextProcessor::new(
            RootAssetTransformer::new(AddText(" processed".into())),
            CoolTextSaver,
        ))
        .set_default_asset_processor::<AddTextProcessor>("cool.ron");

    let guard = source_gate.write_blocking();

    // All the assets will have the same path, but they will still be separately processed since
    // they are in different sources.
    let path = Path::new("asset.cool.ron");
    default_source_dir.insert_asset_text(path, &serialize_as_cool_text("default asset"));
    custom_1_source_dir.insert_asset_text(path, &serialize_as_cool_text("custom 1 asset"));
    custom_2_source_dir.insert_asset_text(path, &serialize_as_cool_text("custom 2 asset"));

    run_app_until_finished_processing(&mut app, guard);

    // Check that all the assets are processed.
    assert_eq!(
        read_asset_as_string(&default_processed_dir, path),
        serialize_as_cool_text("default asset processed")
    );
    assert_eq!(
        read_asset_as_string(&custom_1_processed_dir, path),
        serialize_as_cool_text("custom 1 asset processed")
    );
    assert_eq!(
        read_asset_as_string(&custom_2_processed_dir, path),
        serialize_as_cool_text("custom 2 asset processed")
    );

    let guard = source_gate.write_blocking();

    // Update the default source asset and notify the watcher.
    default_source_dir.insert_asset_text(path, &serialize_as_cool_text("default asset changed"));
    default_source_events
        .send_blocking(AssetSourceEvent::ModifiedAsset(path.to_path_buf()))
        .unwrap();

    run_app_until_finished_processing(&mut app, guard);

    // Check that all the assets are processed again.
    assert_eq!(
        read_asset_as_string(&default_processed_dir, path),
        serialize_as_cool_text("default asset changed processed")
    );
    assert_eq!(
        read_asset_as_string(&custom_1_processed_dir, path),
        serialize_as_cool_text("custom 1 asset processed")
    );
    assert_eq!(
        read_asset_as_string(&custom_2_processed_dir, path),
        serialize_as_cool_text("custom 2 asset processed")
    );

    let guard = source_gate.write_blocking();

    // Update the custom source assets and notify the watchers.
    custom_1_source_dir.insert_asset_text(path, &serialize_as_cool_text("custom 1 asset changed"));
    custom_2_source_dir.insert_asset_text(path, &serialize_as_cool_text("custom 2 asset changed"));
    custom_1_source_events
        .send_blocking(AssetSourceEvent::ModifiedAsset(path.to_path_buf()))
        .unwrap();
    custom_2_source_events
        .send_blocking(AssetSourceEvent::ModifiedAsset(path.to_path_buf()))
        .unwrap();

    run_app_until_finished_processing(&mut app, guard);

    // Check that all the assets are processed again.
    assert_eq!(
        read_asset_as_string(&default_processed_dir, path),
        serialize_as_cool_text("default asset changed processed")
    );
    assert_eq!(
        read_asset_as_string(&custom_1_processed_dir, path),
        serialize_as_cool_text("custom 1 asset changed processed")
    );
    assert_eq!(
        read_asset_as_string(&custom_2_processed_dir, path),
        serialize_as_cool_text("custom 2 asset changed processed")
    );
}

#[test]
fn nested_loads_of_processed_asset_reprocesses_on_reload() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: default_source_dir,
                processed: default_processed_dir,
                source_event_sender: default_source_events,
            },
        extra_sources_dirs,
    } = create_app_with_asset_processor(&["custom".into()]);
    let ProcessingDirs {
        source: custom_source_dir,
        processed: custom_processed_dir,
        source_event_sender: custom_source_events,
    } = extra_sources_dirs["custom"].clone();

    #[derive(Serialize, Deserialize)]
    enum NesterSerialized {
        Leaf(String),
        Path(String),
    }

    #[derive(Asset, TypePath)]
    struct Nester {
        value: String,
    }

    #[derive(TypePath)]
    struct NesterLoader;

    impl AssetLoader for NesterLoader {
        type Asset = Nester;
        type Settings = ();
        type Error = std::io::Error;

        async fn load(
            &self,
            reader: &mut dyn Reader,
            _settings: &Self::Settings,
            load_context: &mut LoadContext<'_>,
        ) -> Result<Self::Asset, Self::Error> {
            let mut bytes = vec![];
            reader.read_to_end(&mut bytes).await?;

            let serialized: NesterSerialized = ron::de::from_bytes(&bytes).unwrap();
            Ok(match serialized {
                NesterSerialized::Leaf(value) => Nester { value },
                NesterSerialized::Path(path) => {
                    let loaded_asset = load_context.loader().immediate().load(path).await.unwrap();
                    loaded_asset.take()
                }
            })
        }

        fn extensions(&self) -> &[&str] {
            &["nest"]
        }
    }

    #[derive(TypePath)]
    struct AddTextToNested(String, Arc<Mutex<u32>>);

    impl MutateAsset<Nester> for AddTextToNested {
        fn mutate(&self, asset: &mut Nester) {
            asset.value.push_str(&self.0);

            *self.1.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        }
    }

    fn serialize_as_leaf(value: String) -> String {
        let serialized = NesterSerialized::Leaf(value);
        ron::ser::to_string(&serialized).unwrap()
    }

    #[derive(TypePath)]
    struct NesterSaver;

    impl AssetSaver for NesterSaver {
        type Asset = Nester;
        type Error = std::io::Error;
        type Settings = ();
        type OutputLoader = NesterLoader;

        async fn save(
            &self,
            writer: &mut crate::io::Writer,
            asset: crate::saver::SavedAsset<'_, Self::Asset>,
            _settings: &Self::Settings,
        ) -> Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error> {
            let serialized = serialize_as_leaf(asset.get().value.clone());
            writer.write_all(serialized.as_bytes()).await
        }
    }

    let process_counter = Arc::new(Mutex::new(0));

    type NesterProcessor = LoadTransformAndSave<
        NesterLoader,
        RootAssetTransformer<AddTextToNested, Nester>,
        NesterSaver,
    >;
    app.init_asset::<Nester>()
        .register_asset_loader(NesterLoader)
        .register_asset_processor(NesterProcessor::new(
            RootAssetTransformer::new(AddTextToNested("-ref".into(), process_counter.clone())),
            NesterSaver,
        ))
        .set_default_asset_processor::<NesterProcessor>("nest");

    let guard = source_gate.write_blocking();

    // This test also checks that processing of nested assets can occur across asset sources.
    custom_source_dir.insert_asset_text(
        Path::new("top.nest"),
        &ron::ser::to_string(&NesterSerialized::Path("middle.nest".into())).unwrap(),
    );
    default_source_dir.insert_asset_text(
        Path::new("middle.nest"),
        &ron::ser::to_string(&NesterSerialized::Path("custom://bottom.nest".into())).unwrap(),
    );
    custom_source_dir
        .insert_asset_text(Path::new("bottom.nest"), &serialize_as_leaf("leaf".into()));
    default_source_dir.insert_asset_text(
        Path::new("unrelated.nest"),
        &serialize_as_leaf("unrelated".into()),
    );

    run_app_until_finished_processing(&mut app, guard);

    // The initial processing step should have processed all assets.
    assert_eq!(
        read_asset_as_string(&custom_processed_dir, Path::new("bottom.nest")),
        serialize_as_leaf("leaf-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, Path::new("middle.nest")),
        serialize_as_leaf("leaf-ref-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&custom_processed_dir, Path::new("top.nest")),
        serialize_as_leaf("leaf-ref-ref-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, Path::new("unrelated.nest")),
        serialize_as_leaf("unrelated-ref".into())
    );

    let get_process_count = || {
        *process_counter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    };
    assert_eq!(get_process_count(), 4);

    // Now we will only send a single source event, but that should still result in all related
    // assets being reprocessed.
    let guard = source_gate.write_blocking();

    custom_source_dir.insert_asset_text(
        Path::new("bottom.nest"),
        &serialize_as_leaf("leaf changed".into()),
    );
    custom_source_events
        .send_blocking(AssetSourceEvent::ModifiedAsset("bottom.nest".into()))
        .unwrap();

    run_app_until_finished_processing(&mut app, guard);

    assert_eq!(
        read_asset_as_string(&custom_processed_dir, Path::new("bottom.nest")),
        serialize_as_leaf("leaf changed-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, Path::new("middle.nest")),
        serialize_as_leaf("leaf changed-ref-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&custom_processed_dir, Path::new("top.nest")),
        serialize_as_leaf("leaf changed-ref-ref-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, Path::new("unrelated.nest")),
        serialize_as_leaf("unrelated-ref".into())
    );

    assert_eq!(get_process_count(), 7);

    // Send a modify event to the middle asset without changing the asset bytes. This should do
    // **nothing** since neither its dependencies nor its bytes have changed.
    let guard = source_gate.write_blocking();

    default_source_events
        .send_blocking(AssetSourceEvent::ModifiedAsset("middle.nest".into()))
        .unwrap();

    run_app_until_finished_processing(&mut app, guard);

    assert_eq!(
        read_asset_as_string(&custom_processed_dir, Path::new("bottom.nest")),
        serialize_as_leaf("leaf changed-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, Path::new("middle.nest")),
        serialize_as_leaf("leaf changed-ref-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&custom_processed_dir, Path::new("top.nest")),
        serialize_as_leaf("leaf changed-ref-ref-ref".into())
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, Path::new("unrelated.nest")),
        serialize_as_leaf("unrelated-ref".into())
    );

    assert_eq!(get_process_count(), 7);
}

#[test]
fn clears_invalid_data_from_processed_dir() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: default_source_dir,
                processed: default_processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;
    app.init_asset::<CoolText>()
        .init_asset::<SubText>()
        .register_asset_loader(CoolTextLoader)
        .register_asset_processor(CoolTextProcessor::new(
            RootAssetTransformer::new(AddText(" processed".to_string())),
            CoolTextSaver,
        ))
        .set_default_asset_processor::<CoolTextProcessor>("cool.ron");

    let guard = source_gate.write_blocking();

    default_source_dir.insert_asset_text(Path::new("a.cool.ron"), &serialize_as_cool_text("a"));
    default_source_dir.insert_asset_text(Path::new("dir/b.cool.ron"), &serialize_as_cool_text("b"));
    default_source_dir.insert_asset_text(
        Path::new("dir/subdir/c.cool.ron"),
        &serialize_as_cool_text("c"),
    );

    // This asset has the right data, but no meta, so it should be reprocessed.
    let a = Path::new("a.cool.ron");
    default_processed_dir.insert_asset_text(a, &serialize_as_cool_text("a processed"));
    // These assets aren't present in the unprocessed directory, so they should be deleted.
    let missing1 = Path::new("missing1.cool.ron");
    let missing2 = Path::new("dir/missing2.cool.ron");
    let missing3 = Path::new("other_dir/missing3.cool.ron");
    default_processed_dir.insert_asset_text(missing1, &serialize_as_cool_text("missing1"));
    default_processed_dir.insert_meta_text(missing1, ""); // This asset has metadata.
    default_processed_dir.insert_asset_text(missing2, &serialize_as_cool_text("missing2"));
    default_processed_dir.insert_asset_text(missing3, &serialize_as_cool_text("missing3"));
    // This directory is empty, so it should be deleted.
    let empty_dir = Path::new("empty_dir");
    let empty_dir_subdir = Path::new("empty_dir/empty_subdir");
    default_processed_dir.get_or_insert_dir(empty_dir_subdir);

    run_app_until_finished_processing(&mut app, guard);

    assert_eq!(
        read_asset_as_string(&default_processed_dir, a),
        serialize_as_cool_text("a processed")
    );
    assert!(default_processed_dir.get_metadata(a).is_some());

    assert!(default_processed_dir.get_asset(missing1).is_none());
    assert!(default_processed_dir.get_metadata(missing1).is_none());
    assert!(default_processed_dir.get_asset(missing2).is_none());
    assert!(default_processed_dir.get_asset(missing3).is_none());

    assert!(default_processed_dir.get_dir(empty_dir_subdir).is_none());
    assert!(default_processed_dir.get_dir(empty_dir).is_none());
}

#[test]
fn only_reprocesses_wrong_hash_on_startup() {
    let no_deps_asset = Path::new("no_deps.cool.ron");
    let source_changed_asset = Path::new("source_changed.cool.ron");
    let dep_unchanged_asset = Path::new("dep_unchanged.cool.ron");
    let dep_changed_asset = Path::new("dep_changed.cool.ron");
    let multi_unchanged_asset = Path::new("multi_unchanged.gltf");
    let multi_changed_asset = Path::new("multi_changed.gltf");
    let default_source_dir;
    let default_processed_dir;

    #[derive(TypePath, Clone)]
    struct MergeEmbeddedAndAddText;

    impl MutateAsset<CoolText> for MergeEmbeddedAndAddText {
        fn mutate(&self, asset: &mut CoolText) {
            asset.text.push_str(" processed");
            if asset.embedded.is_empty() {
                return;
            }
            asset.text.push(' ');
            asset.text.push_str(&asset.embedded);
        }
    }

    #[derive(TypePath, Clone)]
    struct Count<P>(Arc<Mutex<u32>>, P);

    impl<P: Process> Process for Count<P> {
        type Settings = P::Settings;

        async fn process(
            &self,
            context: &mut ProcessContext<'_>,
            settings: &Self::Settings,
            writer_context: WriterContext<'_>,
        ) -> Result<(), ProcessError> {
            *self.0.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            self.1.process(context, settings, writer_context).await
        }
    }

    let counter = Arc::new(Mutex::new(0));
    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<MergeEmbeddedAndAddText, CoolText>,
        CoolTextSaver,
    >;

    /// Assert that the `unsplit_path` gets split with `subpath` to contain a [`FakeGltf`] with just
    /// one mesh.
    fn assert_split_gltf(dir: &Dir, unsplit_path: &Path, subpath: &str, data: &str) {
        assert_eq!(
            read_asset_as_string(dir, &unsplit_path.join(subpath)),
            serialize_gltf_to_string(&FakeGltf {
                gltf_nodes: Default::default(),
                gltf_meshes: vec![data.into()]
            })
        );
    }

    // Create a scope so that the app is completely gone afterwards (and we can see what happens
    // after reinitializing).
    {
        let AppWithProcessor {
            mut app,
            source_gate,
            default_source_dirs,
            ..
        } = create_app_with_asset_processor(&[]);
        default_source_dir = default_source_dirs.source;
        default_processed_dir = default_source_dirs.processed;

        app.init_asset::<CoolText>()
            .init_asset::<SubText>()
            .init_asset::<FakeGltf>()
            .register_asset_loader(CoolTextLoader)
            .register_asset_processor(Count(
                counter.clone(),
                CoolTextProcessor::new(
                    RootAssetTransformer::new(MergeEmbeddedAndAddText),
                    CoolTextSaver,
                ),
            ))
            .set_default_asset_processor::<Count<CoolTextProcessor>>("cool.ron")
            .register_asset_loader(FakeGltfLoader)
            .register_asset_processor(Count(counter.clone(), FakeGltfSplitProcessor))
            .set_default_asset_processor::<Count<FakeGltfSplitProcessor>>("gltf");

        let guard = source_gate.write_blocking();

        let cool_text_with_embedded = |text: &str, embedded: &Path| {
            let cool_text_ron = CoolTextRon {
                text: text.into(),
                dependencies: vec![],
                embedded_dependencies: vec![embedded.to_string_lossy().into_owned()],
                sub_texts: vec![],
            };
            ron::ser::to_string_pretty(&cool_text_ron, PrettyConfig::new().new_line("\n")).unwrap()
        };

        default_source_dir.insert_asset_text(no_deps_asset, &serialize_as_cool_text("no_deps"));
        default_source_dir.insert_asset_text(
            source_changed_asset,
            &serialize_as_cool_text("source_changed"),
        );
        default_source_dir.insert_asset_text(
            dep_unchanged_asset,
            &cool_text_with_embedded("dep_unchanged", no_deps_asset),
        );
        default_source_dir.insert_asset_text(
            dep_changed_asset,
            &cool_text_with_embedded("dep_changed", source_changed_asset),
        );

        default_source_dir.insert_asset_text(
            multi_unchanged_asset,
            &serialize_gltf_to_string(&FakeGltf {
                gltf_nodes: Default::default(),
                gltf_meshes: vec!["a1".into(), "a2".into(), "a3".into()],
            }),
        );
        default_source_dir.insert_asset_text(
            multi_changed_asset,
            &serialize_gltf_to_string(&FakeGltf {
                gltf_nodes: Default::default(),
                gltf_meshes: vec!["b1".into(), "b2".into()],
            }),
        );

        run_app_until_finished_processing(&mut app, guard);

        assert_eq!(
            read_asset_as_string(&default_processed_dir, no_deps_asset),
            serialize_as_cool_text("no_deps processed")
        );
        assert_eq!(
            read_asset_as_string(&default_processed_dir, source_changed_asset),
            serialize_as_cool_text("source_changed processed")
        );
        assert_eq!(
            read_asset_as_string(&default_processed_dir, dep_unchanged_asset),
            serialize_as_cool_text("dep_unchanged processed no_deps processed")
        );
        assert_eq!(
            read_asset_as_string(&default_processed_dir, dep_changed_asset),
            serialize_as_cool_text("dep_changed processed source_changed processed")
        );

        assert_split_gltf(
            &default_processed_dir,
            multi_unchanged_asset,
            "Mesh0.gltf",
            "a1",
        );
        assert_split_gltf(
            &default_processed_dir,
            multi_unchanged_asset,
            "Mesh1.gltf",
            "a2",
        );
        assert_split_gltf(
            &default_processed_dir,
            multi_unchanged_asset,
            "Mesh2.gltf",
            "a3",
        );

        assert_split_gltf(
            &default_processed_dir,
            multi_changed_asset,
            "Mesh0.gltf",
            "b1",
        );
        assert_split_gltf(
            &default_processed_dir,
            multi_changed_asset,
            "Mesh1.gltf",
            "b2",
        );
    }

    // Assert and reset the processing count.
    assert_eq!(
        core::mem::take(&mut *counter.lock().unwrap_or_else(PoisonError::into_inner)),
        6
    );

    // Hand-make the app, since we need to pass in our already existing Dirs from the last app.
    let mut app = App::new();
    let source_gate = Arc::new(RwLock::new(()));

    let source_memory_reader = LockGatedReader::new(
        source_gate.clone(),
        MemoryAssetReader {
            root: default_source_dir.clone(),
        },
    );
    let processed_memory_reader = MemoryAssetReader {
        root: default_processed_dir.clone(),
    };
    let processed_memory_writer = MemoryAssetWriter {
        root: default_processed_dir.clone(),
    };

    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || Box::new(source_memory_reader.clone()))
            .with_processed_reader(move || Box::new(processed_memory_reader.clone()))
            .with_processed_writer(move |_| Some(Box::new(processed_memory_writer.clone()))),
    );

    app.add_plugins((
        TaskPoolPlugin::default(),
        AssetPlugin {
            mode: AssetMode::Processed,
            use_asset_processor_override: Some(true),
            watch_for_changes_override: Some(true),
            ..Default::default()
        },
    ));

    set_fake_transaction_log(&mut app);

    app.init_asset::<CoolText>()
        .init_asset::<SubText>()
        .register_asset_loader(CoolTextLoader)
        .register_asset_processor(Count(
            counter.clone(),
            CoolTextProcessor::new(
                RootAssetTransformer::new(MergeEmbeddedAndAddText),
                CoolTextSaver,
            ),
        ))
        .set_default_asset_processor::<Count<CoolTextProcessor>>("cool.ron")
        .register_asset_loader(FakeGltfLoader)
        .register_asset_processor(Count(counter.clone(), FakeGltfSplitProcessor))
        .set_default_asset_processor::<Count<FakeGltfSplitProcessor>>("gltf");

    let guard = source_gate.write_blocking();

    default_source_dir
        .insert_asset_text(source_changed_asset, &serialize_as_cool_text("DIFFERENT"));
    default_source_dir.insert_asset_text(
        multi_changed_asset,
        &serialize_gltf_to_string(&FakeGltf {
            gltf_nodes: Default::default(),
            gltf_meshes: vec!["c1".into()],
        }),
    );

    run_app_until_finished_processing(&mut app, guard);

    // Only source_changed and dep_changed assets were reprocessed - all others still have the same
    // hashes.
    let num_processes = *counter.lock().unwrap_or_else(PoisonError::into_inner);
    // TODO: assert_eq! (num_processes == 3) only after we prevent double processing assets
    // == 4 happens when the initial processing of an asset and the re-processing that its dependency
    // triggers are both able to proceed. (dep_changed_asset in this case is processed twice)
    assert!(num_processes == 3 || num_processes == 4);

    assert_eq!(
        read_asset_as_string(&default_processed_dir, no_deps_asset),
        serialize_as_cool_text("no_deps processed")
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, source_changed_asset),
        serialize_as_cool_text("DIFFERENT processed")
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, dep_unchanged_asset),
        serialize_as_cool_text("dep_unchanged processed no_deps processed")
    );
    assert_eq!(
        read_asset_as_string(&default_processed_dir, dep_changed_asset),
        serialize_as_cool_text("dep_changed processed DIFFERENT processed")
    );

    assert_split_gltf(
        &default_processed_dir,
        multi_unchanged_asset,
        "Mesh0.gltf",
        "a1",
    );
    assert_split_gltf(
        &default_processed_dir,
        multi_unchanged_asset,
        "Mesh1.gltf",
        "a2",
    );
    assert_split_gltf(
        &default_processed_dir,
        multi_unchanged_asset,
        "Mesh2.gltf",
        "a3",
    );

    assert_split_gltf(
        &default_processed_dir,
        multi_changed_asset,
        "Mesh0.gltf",
        "c1",
    );
    // The multi-processing should have deleted the previous files.
    assert!(default_processed_dir
        .get_asset(&multi_changed_asset.join("Mesh1.gltf"))
        .is_none());
}

/// Serializes the provided `gltf` into a string (pretty-ly).
fn serialize_gltf_to_string(gltf: &FakeGltf) -> String {
    ron::ser::to_string_pretty(gltf, PrettyConfig::new().new_line("\n"))
        .expect("Conversion is safe")
}

#[test]
fn writes_default_meta_for_processor() {
    let AppWithProcessor {
        mut app,
        default_source_dirs: ProcessingDirs { source, .. },
        ..
    } = create_app_with_asset_processor(&[]);

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;

    app.register_asset_processor(CoolTextProcessor::new(
        RootAssetTransformer::new(AddText("blah".to_string())),
        CoolTextSaver,
    ))
    .set_default_asset_processor::<CoolTextProcessor>("cool.ron");

    const ASSET_PATH: &str = "abc.cool.ron";
    source.insert_asset_text(Path::new(ASSET_PATH), &serialize_as_cool_text("blah"));

    let processor = app.world().resource::<AssetProcessor>().clone();
    bevy_tasks::block_on(processor.write_default_meta_file_for_path(ASSET_PATH)).unwrap();

    assert_eq!(
        read_meta_as_string(&source, Path::new(ASSET_PATH)),
        r#"(
    meta_format_version: "1.0",
    asset: Process(
        processor: "bevy_asset::processor::process::LoadTransformAndSave<bevy_asset::tests::CoolTextLoader, bevy_asset::processor::tests::RootAssetTransformer<bevy_asset::processor::tests::AddText, bevy_asset::tests::CoolText>, bevy_asset::processor::tests::CoolTextSaver>",
        settings: (
            loader_settings: (),
            transformer_settings: (),
            saver_settings: (),
        ),
    ),
)"#
    );
}

#[test]
fn write_default_meta_does_not_overwrite() {
    let AppWithProcessor {
        mut app,
        default_source_dirs: ProcessingDirs { source, .. },
        ..
    } = create_app_with_asset_processor(&[]);

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;

    app.register_asset_processor(CoolTextProcessor::new(
        RootAssetTransformer::new(AddText("blah".to_string())),
        CoolTextSaver,
    ))
    .set_default_asset_processor::<CoolTextProcessor>("cool.ron");

    const ASSET_PATH: &str = "abc.cool.ron";
    source.insert_asset_text(Path::new(ASSET_PATH), &serialize_as_cool_text("blah"));
    const META_TEXT: &str = "hey i'm walkin here!";
    source.insert_meta_text(Path::new(ASSET_PATH), META_TEXT);

    let processor = app.world().resource::<AssetProcessor>().clone();
    assert!(matches!(
        bevy_tasks::block_on(processor.write_default_meta_file_for_path(ASSET_PATH)),
        Err(WriteDefaultMetaError::MetaAlreadyExists)
    ));

    assert_eq!(
        read_meta_as_string(&source, Path::new(ASSET_PATH)),
        META_TEXT
    );
}

#[test]
fn gates_asset_path_on_process() {
    let AppWithProcessor {
        mut app,
        default_source_dirs:
            ProcessingDirs {
                source: default_source_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    /// Gates processing on acquiring the provided lock.
    ///
    /// This has different behavior from [`LockGatedReader`]: [`LockGatedReader`] blocks the
    /// processor from even initializing, and asset loads block on initialization before asset. By
    /// blocking during processing, we ensure that the loader is actually blocking on the processing
    /// of the particular path.
    #[derive(TypePath)]
    struct GatedProcess<P>(Arc<async_lock::Mutex<()>>, P);

    impl<P: Process> Process for GatedProcess<P> {
        type Settings = P::Settings;

        async fn process(
            &self,
            context: &mut ProcessContext<'_>,
            settings: &Self::Settings,
            writer_context: WriterContext<'_>,
        ) -> Result<(), ProcessError> {
            let _guard = self.0.lock().await;
            self.1.process(context, settings, writer_context).await
        }
    }

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;

    let process_gate = Arc::new(async_lock::Mutex::new(()));
    app.init_asset::<CoolText>()
        .init_asset::<SubText>()
        .register_asset_loader(CoolTextLoader)
        .register_asset_processor::<GatedProcess<CoolTextProcessor>>(GatedProcess(
            process_gate.clone(),
            CoolTextProcessor::new(
                RootAssetTransformer::new(AddText(" processed".into())),
                CoolTextSaver,
            ),
        ))
        .set_default_asset_processor::<GatedProcess<CoolTextProcessor>>("cool.ron")
        .init_asset::<FakeGltf>()
        .register_asset_loader(FakeGltfLoader)
        .register_asset_processor(GatedProcess(process_gate.clone(), FakeGltfSplitProcessor))
        .set_default_asset_processor::<GatedProcess<FakeGltfSplitProcessor>>("gltf");

    // Lock the process gate so that we can't complete processing.
    let guard = process_gate.lock_blocking();

    default_source_dir.insert_asset_text(Path::new("abc.cool.ron"), &serialize_as_cool_text("abc"));
    default_source_dir.insert_asset_text(
        Path::new("def.gltf"),
        &serialize_gltf_to_string(&FakeGltf {
            gltf_nodes: Default::default(),
            gltf_meshes: vec!["a".into(), "b".into()],
        }),
    );

    let processor = app.world().resource::<AssetProcessor>().clone();
    run_app_until(&mut app, |_| {
        (bevy_tasks::block_on(processor.get_state()) == ProcessorState::Processing).then_some(())
    });

    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<CoolText>("abc.cool.ron");
    let handle_multi_a = app
        .world()
        .resource::<AssetServer>()
        .load::<FakeGltf>("def.gltf/Mesh0.gltf");
    let handle_multi_b = app
        .world()
        .resource::<AssetServer>()
        .load::<FakeGltf>("def.gltf/Mesh1.gltf");
    // Update an arbitrary number of times. If at any point, the asset loads, we know we're not
    // blocked on processing the asset! Note: If we're not blocking on the processed asset (this
    // feature is broken), this test would be flaky on multi_threaded (though it should still
    // deterministically fail on single-threaded).
    for _ in 0..100 {
        app.update();
        assert!(app
            .world()
            .resource::<Assets<CoolText>>()
            .get(&handle)
            .is_none());
    }

    // Now processing can finish!
    drop(guard);
    // Wait until the asset finishes loading, now that we're not blocked on the processor.
    run_app_until(&mut app, |world| {
        // Return None if any of these assets are still missing.
        world.resource::<Assets<CoolText>>().get(&handle)?;
        world.resource::<Assets<FakeGltf>>().get(&handle_multi_a)?;
        world.resource::<Assets<FakeGltf>>().get(&handle_multi_b)?;
        Some(())
    });

    assert_eq!(
        app.world()
            .resource::<Assets<CoolText>>()
            .get(&handle)
            .unwrap()
            .text,
        "abc processed"
    );
    let gltfs = app.world().resource::<Assets<FakeGltf>>();
    assert_eq!(
        gltfs.get(&handle_multi_a).unwrap().gltf_meshes,
        ["a".to_string()]
    );
    assert_eq!(
        gltfs.get(&handle_multi_b).unwrap().gltf_meshes,
        ["b".to_string()]
    );
}

/// A processor for [`FakeGltf`] that splits each mesh into its own [`FakeGltf`] file, and its nodes
/// into a [`FakeBsn`] file.
#[derive(TypePath)]
struct FakeGltfSplitProcessor;

impl Process for FakeGltfSplitProcessor {
    type Settings = ();

    async fn process(
        &self,
        context: &mut ProcessContext<'_>,
        _settings: &Self::Settings,
        writer_context: WriterContext<'_>,
    ) -> Result<(), ProcessError> {
        use ron::ser::PrettyConfig;

        use crate::io::AssetWriterError;

        let gltf = context.load_source_asset::<FakeGltfLoader>(&()).await?;
        let Ok(gltf) = gltf.downcast::<FakeGltf>() else {
            panic!("It should be impossible to downcast to the wrong type here")
        };

        let root_path = context.path().clone_owned();

        let gltf = gltf.take();
        for (index, buffer) in gltf.gltf_meshes.into_iter().enumerate() {
            let mut writer = writer_context
                .write_multiple(Path::new(&format!("Mesh{index}.gltf")))
                .await?;
            let mesh_data = serialize_gltf_to_string(&FakeGltf {
                gltf_meshes: vec![buffer],
                gltf_nodes: Default::default(),
            });
            writer
                .write_all(mesh_data.as_bytes())
                .await
                .map_err(|err| ProcessError::AssetWriterError {
                    path: root_path.clone_owned(),
                    err: AssetWriterError::Io(err),
                })?;
            writer.finish::<FakeGltfLoader>(()).await?;
        }

        let mut writer = writer_context
            .write_multiple(Path::new("Scene0.bsn"))
            .await?;
        let scene_data = ron::ser::to_string_pretty(
            &FakeBsn {
                parent_bsn: None,
                nodes: gltf.gltf_nodes,
            },
            PrettyConfig::new().new_line("\n"),
        )
        .expect("Conversion is safe");
        writer
            .write_all(scene_data.as_bytes())
            .await
            .map_err(|err| ProcessError::AssetWriterError {
                path: root_path.clone_owned(),
                err: AssetWriterError::Io(err),
            })?;
        writer.finish::<FakeBsnLoader>(()).await?;
        Ok(())
    }
}

#[test]
fn asset_processor_can_write_multiple_files() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    app.register_asset_loader(FakeGltfLoader)
        .register_asset_loader(FakeBsnLoader)
        .register_asset_processor(FakeGltfSplitProcessor)
        .set_default_asset_processor::<FakeGltfSplitProcessor>("gltf");

    let guard = source_gate.write_blocking();

    let gltf_path = Path::new("abc.gltf");
    source_dir.insert_asset_text(
        gltf_path,
        r#"(
    gltf_nodes: {
        "name": "thing",
        "position": "123",
    },
    gltf_meshes: ["buffer1", "buffer2", "buffer3"],
)"#,
    );

    run_app_until_finished_processing(&mut app, guard);

    let path_to_data = |path| {
        let data = processed_dir.get_asset(Path::new(path)).unwrap();
        let data = str::from_utf8(data.value()).unwrap();
        data.to_string()
    };

    // All the meshes were decomposed into separate asset files.
    assert_eq!(
        path_to_data("abc.gltf/Mesh0.gltf"),
        r#"(
    gltf_nodes: {},
    gltf_meshes: [
        "buffer1",
    ],
)"#
    );
    assert_eq!(
        path_to_data("abc.gltf/Mesh1.gltf"),
        r#"(
    gltf_nodes: {},
    gltf_meshes: [
        "buffer2",
    ],
)"#
    );
    assert_eq!(
        path_to_data("abc.gltf/Mesh2.gltf"),
        r#"(
    gltf_nodes: {},
    gltf_meshes: [
        "buffer3",
    ],
)"#
    );

    // The nodes should have been written to the scene file.
    assert_eq!(
        path_to_data("abc.gltf/Scene0.bsn"),
        r#"(
    parent_bsn: None,
    nodes: {
        "name": "thing",
        "position": "123",
    },
)"#
    );
}

#[test]
fn error_on_no_writer() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs: ProcessingDirs {
            source: source_dir, ..
        },
        ..
    } = create_app_with_asset_processor(&[]);

    #[derive(TypePath)]
    struct NoWriterProcess;

    impl Process for NoWriterProcess {
        type Settings = ();

        async fn process(
            &self,
            _: &mut ProcessContext<'_>,
            _: &Self::Settings,
            _: WriterContext<'_>,
        ) -> Result<(), ProcessError> {
            // Don't start a writer!
            Ok(())
        }
    }

    app.register_asset_processor(NoWriterProcess)
        .set_default_asset_processor::<NoWriterProcess>("txt");

    let guard = source_gate.write_blocking();
    source_dir.insert_asset_text(Path::new("whatever.txt"), "");

    run_app_until_finished_processing(&mut app, guard);

    let process_status = bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_processed("whatever.txt".into()),
    );
    // The process failed due to not having a writer.
    assert_eq!(process_status, ProcessStatus::Failed);
}

#[test]
fn error_on_unfinished_writer() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs: ProcessingDirs {
            source: source_dir, ..
        },
        ..
    } = create_app_with_asset_processor(&[]);

    #[derive(TypePath)]
    struct UnfinishedWriterProcess;

    impl Process for UnfinishedWriterProcess {
        type Settings = ();

        async fn process(
            &self,
            _: &mut ProcessContext<'_>,
            _: &Self::Settings,
            writer_context: WriterContext<'_>,
        ) -> Result<(), ProcessError> {
            let _writer = writer_context.write_single().await?;
            // Don't call finish on the writer!
            Ok(())
        }
    }

    app.register_asset_processor(UnfinishedWriterProcess)
        .set_default_asset_processor::<UnfinishedWriterProcess>("txt");

    let guard = source_gate.write_blocking();
    source_dir.insert_asset_text(Path::new("whatever.txt"), "");

    run_app_until_finished_processing(&mut app, guard);

    let process_status = bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_processed("whatever.txt".into()),
    );
    // The process failed due to having a writer that we didn't await finish on.
    assert_eq!(process_status, ProcessStatus::Failed);
}

#[test]
fn error_on_single_writer_after_multiple_writer() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs: ProcessingDirs {
            source: source_dir, ..
        },
        ..
    } = create_app_with_asset_processor(&[]);

    #[derive(TypePath)]
    struct SingleAfterMultipleWriterProcess;

    impl Process for SingleAfterMultipleWriterProcess {
        type Settings = ();

        async fn process(
            &self,
            _: &mut ProcessContext<'_>,
            _: &Self::Settings,
            writer_context: WriterContext<'_>,
        ) -> Result<(), ProcessError> {
            // Properly write a "multiple".
            let writer = writer_context
                .write_multiple(Path::new("multi.txt"))
                .await?;
            writer.finish::<CoolTextLoader>(()).await?;

            // Now trying writing "single", which conflicts!
            let writer = writer_context.write_single().await?;
            writer.finish::<CoolTextLoader>(()).await?;

            Ok(())
        }
    }

    app.register_asset_processor(SingleAfterMultipleWriterProcess)
        .set_default_asset_processor::<SingleAfterMultipleWriterProcess>("txt");

    let guard = source_gate.write_blocking();
    source_dir.insert_asset_text(Path::new("whatever.txt"), "");

    run_app_until_finished_processing(&mut app, guard);

    let process_status = bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_processed("whatever.txt".into()),
    );
    // The process failed due to having a single writer after a multiple writer.
    assert_eq!(process_status, ProcessStatus::Failed);
}

#[test]
fn processor_can_parallelize_multiple_writes() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs:
            ProcessingDirs {
                source: source_dir,
                processed: processed_dir,
                ..
            },
        ..
    } = create_app_with_asset_processor(&[]);

    #[derive(TypePath)]
    struct ParallelizedWriterProcess;

    impl Process for ParallelizedWriterProcess {
        type Settings = ();

        async fn process(
            &self,
            _: &mut ProcessContext<'_>,
            _: &Self::Settings,
            writer_context: WriterContext<'_>,
        ) -> Result<(), ProcessError> {
            let mut writer_1 = writer_context.write_multiple(Path::new("a.txt")).await?;
            let mut writer_2 = writer_context.write_multiple(Path::new("b.txt")).await?;

            // Note: this call is blocking, so it's undesirable in production code using
            // single-threaded mode (e.g., platforms like Wasm). For this test though, it's not a
            // big deal.
            bevy_tasks::IoTaskPool::get().scope(|scope| {
                scope.spawn(async {
                    writer_1.write_all(b"abc123").await.unwrap();
                    writer_1.finish::<CoolTextLoader>(()).await.unwrap();
                });
                scope.spawn(async {
                    writer_2.write_all(b"def456").await.unwrap();
                    writer_2.finish::<CoolTextLoader>(()).await.unwrap();
                });
            });

            Ok(())
        }
    }

    app.register_asset_processor(ParallelizedWriterProcess)
        .set_default_asset_processor::<ParallelizedWriterProcess>("txt");

    let guard = source_gate.write_blocking();
    source_dir.insert_asset_text(Path::new("whatever.txt"), "");

    run_app_until_finished_processing(&mut app, guard);

    assert_eq!(
        &read_asset_as_string(&processed_dir, Path::new("whatever.txt/a.txt")),
        "abc123"
    );
    assert_eq!(
        &read_asset_as_string(&processed_dir, Path::new("whatever.txt/b.txt")),
        "def456"
    );
}

#[test]
fn error_on_two_multiple_writes_for_same_path() {
    let AppWithProcessor {
        mut app,
        source_gate,
        default_source_dirs: ProcessingDirs {
            source: source_dir, ..
        },
        ..
    } = create_app_with_asset_processor(&[]);

    #[derive(TypePath)]
    struct TwoMultipleWritesForSamePathProcess;

    impl Process for TwoMultipleWritesForSamePathProcess {
        type Settings = ();

        async fn process(
            &self,
            _: &mut ProcessContext<'_>,
            _: &Self::Settings,
            writer_context: WriterContext<'_>,
        ) -> Result<(), ProcessError> {
            // Properly write a "multiple".
            let writer = writer_context
                .write_multiple(Path::new("multi.txt"))
                .await?;
            writer.finish::<CoolTextLoader>(()).await?;

            // Properly write to the same "multiple".
            let writer = writer_context
                .write_multiple(Path::new("multi.txt"))
                .await?;
            writer.finish::<CoolTextLoader>(()).await?;

            Ok(())
        }
    }

    app.register_asset_processor(TwoMultipleWritesForSamePathProcess)
        .set_default_asset_processor::<TwoMultipleWritesForSamePathProcess>("txt");

    let guard = source_gate.write_blocking();
    source_dir.insert_asset_text(Path::new("whatever.txt"), "");

    run_app_until_finished_processing(&mut app, guard);

    let process_status = bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_processed("whatever.txt".into()),
    );
    // The process failed due to writing "multiple" to the same path twice.
    assert_eq!(process_status, ProcessStatus::Failed);
}
