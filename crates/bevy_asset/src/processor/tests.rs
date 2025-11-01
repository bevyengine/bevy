use alloc::{
    boxed::Box,
    collections::BTreeMap,
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
        AssetReader, AssetReaderError, AssetSource, AssetSourceEvent, AssetSourceId, AssetWatcher,
        PathStream, Reader,
    },
    processor::{
        AssetProcessor, LoadTransformAndSave, LogEntry, ProcessorState, ProcessorTransactionLog,
        ProcessorTransactionLogFactory,
    },
    saver::AssetSaver,
    tests::{run_app_until, CoolText, CoolTextLoader, CoolTextRon, SubText},
    transformer::{AssetTransformer, TransformedAsset},
    Asset, AssetApp, AssetLoader, AssetMode, AssetPath, AssetPlugin, LoadContext,
};

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
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let _guard = self.gate.read().await;
        self.reader.read(path).await
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
            AssetSource::build()
                .with_reader(move || Box::new(source_memory_reader.clone()))
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
        let state = bevy_tasks::block_on(processor.get_state());
        (state == ProcessorState::Processing || state == ProcessorState::Initializing).then_some(())
    });
    drop(guard);
    run_app_until(app, |_| {
        (bevy_tasks::block_on(processor.get_state()) == ProcessorState::Finished).then_some(())
    });
}

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
struct RootAssetTransformer<M: MutateAsset<A>, A: Asset>(M, PhantomData<fn(&mut A)>);

trait MutateAsset<A: Asset>: Send + Sync + 'static {
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

struct AddText(String);

impl MutateAsset<CoolText> for AddText {
    fn mutate(&self, text: &mut CoolText) {
        text.text.push_str(&self.0);
    }
}

fn read_asset_as_string(dir: &Dir, path: &Path) -> String {
    let bytes = dir.get_asset(path).unwrap();
    str::from_utf8(bytes.value()).unwrap().to_string()
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

#[derive(Asset, TypePath, Serialize, Deserialize)]
struct FakeGltf {
    gltf_nodes: BTreeMap<String, String>,
}

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
    }
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
    }
)"#,
    );
    let gltf_path_2 = Path::new("def.gltf");
    source_dir.insert_asset_text(
        gltf_path_2,
        r#"(
    gltf_nodes: {
        "velocity": "456",
        "color": "red",
    }
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
    let serialize_as_cool_text = |text: &str| {
        let cool_text_ron = CoolTextRon {
            text: text.into(),
            dependencies: vec![],
            embedded_dependencies: vec![],
            sub_texts: vec![],
        };
        ron::ser::to_string_pretty(&cool_text_ron, PrettyConfig::new().new_line("\n")).unwrap()
    };
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
