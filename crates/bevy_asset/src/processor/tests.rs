use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use bevy_reflect::TypePath;
use core::marker::PhantomData;
use futures_lite::AsyncWriteExt;
use serde::{Deserialize, Serialize};
use std::path::Path;

use bevy_app::{App, TaskPoolPlugin};
use bevy_ecs::error::BevyError;
use bevy_tasks::BoxedFuture;

use crate::{
    io::{
        memory::{Dir, MemoryAssetReader, MemoryAssetWriter},
        AssetSource, AssetSourceBuilders, AssetSourceId, Reader,
    },
    meta::AssetMeta,
    processor::{
        AssetProcessor, GetProcessorError, LoadTransformAndSave, LogEntry, Process, ProcessContext,
        ProcessError, ProcessorTransactionLog, ProcessorTransactionLogFactory,
    },
    saver::AssetSaver,
    tests::{CoolText, CoolTextLoader, CoolTextRon, SubText},
    transformer::{AssetTransformer, TransformedAsset},
    Asset, AssetApp, AssetLoader, AssetMode, AssetPath, AssetPlugin, LoadContext,
};

#[derive(TypePath)]
struct MyProcessor<T>(PhantomData<fn() -> T>);

impl<T: TypePath + 'static> Process for MyProcessor<T> {
    type OutputLoader = ();
    type Settings = ();

    async fn process(
        &self,
        _context: &mut ProcessContext<'_>,
        _meta: AssetMeta<(), Self>,
        _writer: &mut crate::io::Writer,
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
        AssetSource::build().with_reader(move || Box::new(memory_reader.clone())),
    );

    AssetProcessor::new(&mut sources)
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

struct AppWithProcessor {
    app: App,
    source_dir: Dir,
    processed_dir: Dir,
}

fn create_app_with_asset_processor() -> AppWithProcessor {
    let mut app = App::new();
    let source_dir = Dir::default();
    let processed_dir = Dir::default();

    let source_memory_reader = MemoryAssetReader {
        root: source_dir.clone(),
    };
    let processed_memory_reader = MemoryAssetReader {
        root: processed_dir.clone(),
    };
    let processed_memory_writer = MemoryAssetWriter {
        root: processed_dir.clone(),
    };

    app.register_asset_source(
        AssetSourceId::Default,
        AssetSource::build()
            .with_reader(move || Box::new(source_memory_reader.clone()))
            .with_processed_reader(move || Box::new(processed_memory_reader.clone()))
            .with_processed_writer(move |_| Some(Box::new(processed_memory_writer.clone()))),
    )
    .add_plugins((
        TaskPoolPlugin::default(),
        AssetPlugin {
            mode: AssetMode::Processed,
            use_asset_processor_override: Some(true),
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

    AppWithProcessor {
        app,
        source_dir,
        processed_dir,
    }
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
        let ron = ron::ser::to_string(&ron).unwrap();
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

#[test]
fn no_meta_or_default_processor_copies_asset() {
    // Assets without a meta file or a default processor should still be accessible in the
    // processed path. Note: This isn't exactly the desired property - we don't want the assets
    // to be copied to the processed directory. We just want these assets to still be loadable
    // if we no longer have the source directory. This could be done with a symlink instead of a
    // copy.

    let AppWithProcessor {
        mut app,
        source_dir,
        processed_dir,
    } = create_app_with_asset_processor();

    let path = Path::new("abc.cool.ron");
    let source_asset = r#"(
    text: "abc",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

    source_dir.insert_asset_text(path, source_asset);

    // Start the app, which also starts the asset processor.
    app.update();

    // Wait for all processing to finish.
    bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_finished(),
    );

    let processed_asset = processed_dir.get_asset(path).unwrap();
    let processed_asset = str::from_utf8(processed_asset.value()).unwrap();
    assert_eq!(processed_asset, source_asset);
}

#[test]
fn asset_processor_transforms_asset_default_processor() {
    let AppWithProcessor {
        mut app,
        source_dir,
        processed_dir,
    } = create_app_with_asset_processor();

    #[derive(TypePath)]
    struct AddText;

    impl MutateAsset<CoolText> for AddText {
        fn mutate(&self, text: &mut CoolText) {
            text.text.push_str("_def");
        }
    }

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;
    app.register_asset_loader(CoolTextLoader)
        .register_asset_processor(CoolTextProcessor::new(
            RootAssetTransformer::new(AddText),
            CoolTextSaver,
        ))
        .set_default_asset_processor::<CoolTextProcessor>("cool.ron");

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

    // Start the app, which also starts the asset processor.
    app.update();

    // Wait for all processing to finish.
    bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_finished(),
    );

    let processed_asset = processed_dir.get_asset(path).unwrap();
    let processed_asset = str::from_utf8(processed_asset.value()).unwrap();
    assert_eq!(
        processed_asset,
        r#"(text:"abc_def",dependencies:[],embedded_dependencies:[],sub_texts:[])"#
    );
}

#[test]
fn asset_processor_transforms_asset_with_meta() {
    let AppWithProcessor {
        mut app,
        source_dir,
        processed_dir,
    } = create_app_with_asset_processor();

    #[derive(TypePath)]
    struct AddText;

    impl MutateAsset<CoolText> for AddText {
        fn mutate(&self, text: &mut CoolText) {
            text.text.push_str("_def");
        }
    }

    type CoolTextProcessor = LoadTransformAndSave<
        CoolTextLoader,
        RootAssetTransformer<AddText, CoolText>,
        CoolTextSaver,
    >;
    app.register_asset_loader(CoolTextLoader)
        .register_asset_processor(CoolTextProcessor::new(
            RootAssetTransformer::new(AddText),
            CoolTextSaver,
        ));

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

    // Start the app, which also starts the asset processor.
    app.update();

    // Wait for all processing to finish.
    bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_finished(),
    );

    let processed_asset = processed_dir.get_asset(path).unwrap();
    let processed_asset = str::from_utf8(processed_asset.value()).unwrap();
    assert_eq!(
        processed_asset,
        r#"(text:"abc_def",dependencies:[],embedded_dependencies:[],sub_texts:[])"#
    );
}

#[derive(Asset, TypePath, Serialize, Deserialize)]
struct FakeGltf {
    gltf_nodes: BTreeMap<String, String>,
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

        use ron::ser::PrettyConfig;

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
        source_dir,
        processed_dir,
    } = create_app_with_asset_processor();

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

    // Start the app, which also starts the asset processor.
    app.update();

    // Wait for all processing to finish.
    bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_finished(),
    );

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
        source_dir,
        processed_dir,
    } = create_app_with_asset_processor();

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

    // Start the app, which also starts the asset processor.
    app.update();

    // Wait for all processing to finish.
    bevy_tasks::block_on(
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .wait_until_finished(),
    );

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
