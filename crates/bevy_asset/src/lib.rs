// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

pub mod io;
pub mod meta;
pub mod processor;
pub mod saver;
pub mod transformer;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        Asset, AssetApp, AssetEvent, AssetId, AssetMode, AssetPlugin, AssetServer, Assets,
        DirectAssetAccessExt, Handle, UntypedHandle,
    };
}

mod assets;
mod direct_access_ext;
mod event;
mod folder;
mod handle;
mod id;
mod loader;
mod loader_builders;
mod path;
mod reflect;
mod server;

pub use assets::*;
pub use bevy_asset_macros::Asset;
pub use direct_access_ext::DirectAssetAccessExt;
pub use event::*;
pub use folder::*;
pub use futures_lite::{AsyncReadExt, AsyncWriteExt};
pub use handle::*;
pub use id::*;
pub use loader::*;
pub use loader_builders::{
    DirectNestedLoader, NestedLoader, UntypedDirectNestedLoader, UntypedNestedLoader,
};
pub use path::*;
pub use reflect::*;
pub use server::*;

/// Rusty Object Notation, a crate used to serialize and deserialize bevy assets.
pub use ron;

use crate::{
    io::{embedded::EmbeddedAssetRegistry, AssetSourceBuilder, AssetSourceBuilders, AssetSourceId},
    processor::{AssetProcessor, Process},
};
use bevy_app::{App, Last, Plugin, PreUpdate};
use bevy_ecs::{
    reflect::AppTypeRegistry,
    schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
    world::FromWorld,
};
use bevy_reflect::{FromReflect, GetTypeRegistration, Reflect, TypePath};
use bevy_utils::{tracing::error, HashSet};
use std::{any::TypeId, sync::Arc};

#[cfg(all(feature = "file_watcher", not(feature = "multi_threaded")))]
compile_error!(
    "The \"file_watcher\" feature for hot reloading requires the \
    \"multi_threaded\" feature to be functional.\n\
    Consider either disabling the \"file_watcher\" feature or enabling \"multi_threaded\""
);

/// Provides "asset" loading and processing functionality. An [`Asset`] is a "runtime value" that is loaded from an [`AssetSource`],
/// which can be something like a filesystem, a network, etc.
///
/// Supports flexible "modes", such as [`AssetMode::Processed`] and
/// [`AssetMode::Unprocessed`] that enable using the asset workflow that best suits your project.
///
/// [`AssetSource`]: io::AssetSource
pub struct AssetPlugin {
    /// The default file path to use (relative to the project root) for unprocessed assets.
    pub file_path: String,
    /// The default file path to use (relative to the project root) for processed assets.
    pub processed_file_path: String,
    /// If set, will override the default "watch for changes" setting. By default "watch for changes" will be `false` unless
    /// the `watch` cargo feature is set. `watch` can be enabled manually, or it will be automatically enabled if a specific watcher
    /// like `file_watcher` is enabled.
    ///
    /// Most use cases should leave this set to [`None`] and enable a specific watcher feature such as `file_watcher` to enable
    /// watching for dev-scenarios.
    pub watch_for_changes_override: Option<bool>,
    /// The [`AssetMode`] to use for this server.
    pub mode: AssetMode,
    /// How/If asset meta files should be checked.
    pub meta_check: AssetMetaCheck,
}

#[derive(Debug)]
pub enum AssetMode {
    /// Loads assets from their [`AssetSource`]'s default [`AssetReader`] without any "preprocessing".
    ///
    /// [`AssetReader`]: io::AssetReader
    /// [`AssetSource`]: io::AssetSource
    Unprocessed,
    /// Assets will be "pre-processed". This enables assets to be imported / converted / optimized ahead of time.
    ///
    /// Assets will be read from their unprocessed [`AssetSource`] (defaults to the `assets` folder),
    /// processed according to their [`AssetMeta`], and written to their processed [`AssetSource`] (defaults to the `imported_assets/Default` folder).
    ///
    /// By default, this assumes the processor _has already been run_. It will load assets from their final processed [`AssetReader`].
    ///
    /// When developing an app, you should enable the `asset_processor` cargo feature, which will run the asset processor at startup. This should generally
    /// be used in combination with the `file_watcher` cargo feature, which enables hot-reloading of assets that have changed. When both features are enabled,
    /// changes to "original/source assets" will be detected, the asset will be re-processed, and then the final processed asset will be hot-reloaded in the app.
    ///
    /// [`AssetMeta`]: meta::AssetMeta
    /// [`AssetSource`]: io::AssetSource
    /// [`AssetReader`]: io::AssetReader
    Processed,
}

/// Configures how / if meta files will be checked. If an asset's meta file is not checked, the default meta for the asset
/// will be used.
#[derive(Debug, Default, Clone)]
pub enum AssetMetaCheck {
    /// Always check if assets have meta files. If the meta does not exist, the default meta will be used.
    #[default]
    Always,
    /// Only look up meta files for the provided paths. The default meta will be used for any paths not contained in this set.
    Paths(HashSet<AssetPath<'static>>),
    /// Never check if assets have meta files and always use the default meta. If meta files exist, they will be ignored and the default meta will be used.
    Never,
}

impl Default for AssetPlugin {
    fn default() -> Self {
        Self {
            mode: AssetMode::Unprocessed,
            file_path: Self::DEFAULT_UNPROCESSED_FILE_PATH.to_string(),
            processed_file_path: Self::DEFAULT_PROCESSED_FILE_PATH.to_string(),
            watch_for_changes_override: None,
            meta_check: AssetMetaCheck::default(),
        }
    }
}

impl AssetPlugin {
    const DEFAULT_UNPROCESSED_FILE_PATH: &'static str = "assets";
    /// NOTE: this is in the Default sub-folder to make this forward compatible with "import profiles"
    /// and to allow us to put the "processor transaction log" at `imported_assets/log`
    const DEFAULT_PROCESSED_FILE_PATH: &'static str = "imported_assets/Default";
}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        let embedded = EmbeddedAssetRegistry::default();
        {
            let mut sources = app
                .world_mut()
                .get_resource_or_insert_with::<AssetSourceBuilders>(Default::default);
            sources.init_default_source(
                &self.file_path,
                (!matches!(self.mode, AssetMode::Unprocessed))
                    .then_some(self.processed_file_path.as_str()),
            );
            embedded.register_source(&mut sources);
        }
        {
            let mut watch = cfg!(feature = "watch");
            if let Some(watch_override) = self.watch_for_changes_override {
                watch = watch_override;
            }
            match self.mode {
                AssetMode::Unprocessed => {
                    let mut builders = app.world_mut().resource_mut::<AssetSourceBuilders>();
                    let sources = builders.build_sources(watch, false);

                    app.insert_resource(AssetServer::new_with_meta_check(
                        sources,
                        AssetServerMode::Unprocessed,
                        self.meta_check.clone(),
                        watch,
                    ));
                }
                AssetMode::Processed => {
                    #[cfg(feature = "asset_processor")]
                    {
                        let mut builders = app.world_mut().resource_mut::<AssetSourceBuilders>();
                        let processor = AssetProcessor::new(&mut builders);
                        let mut sources = builders.build_sources(false, watch);
                        sources.gate_on_processor(processor.data.clone());
                        // the main asset server shares loaders with the processor asset server
                        app.insert_resource(AssetServer::new_with_loaders(
                            sources,
                            processor.server().data.loaders.clone(),
                            AssetServerMode::Processed,
                            AssetMetaCheck::Always,
                            watch,
                        ))
                        .insert_resource(processor)
                        .add_systems(bevy_app::Startup, AssetProcessor::start);
                    }
                    #[cfg(not(feature = "asset_processor"))]
                    {
                        let mut builders = app.world_mut().resource_mut::<AssetSourceBuilders>();
                        let sources = builders.build_sources(false, watch);
                        app.insert_resource(AssetServer::new_with_meta_check(
                            sources,
                            AssetServerMode::Processed,
                            AssetMetaCheck::Always,
                            watch,
                        ));
                    }
                }
            }
        }
        app.insert_resource(embedded)
            .init_asset::<LoadedFolder>()
            .init_asset::<LoadedUntypedAsset>()
            .init_asset::<()>()
            .add_event::<UntypedAssetLoadFailedEvent>()
            .configure_sets(PreUpdate, TrackAssets.after(handle_internal_asset_events))
            .add_systems(PreUpdate, handle_internal_asset_events)
            .register_type::<AssetPath>();
    }
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Asset`",
    label = "invalid `Asset`",
    note = "consider annotating `{Self}` with `#[derive(Asset)]`"
)]
pub trait Asset: VisitAssetDependencies + TypePath + Send + Sync + 'static {}

pub trait VisitAssetDependencies {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId));
}

impl<A: Asset> VisitAssetDependencies for Handle<A> {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        visit(self.id().untyped());
    }
}

impl<A: Asset> VisitAssetDependencies for Option<Handle<A>> {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        if let Some(handle) = self {
            visit(handle.id().untyped());
        }
    }
}

impl VisitAssetDependencies for UntypedHandle {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        visit(self.id());
    }
}

impl VisitAssetDependencies for Option<UntypedHandle> {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        if let Some(handle) = self {
            visit(handle.id());
        }
    }
}

impl<A: Asset> VisitAssetDependencies for Vec<Handle<A>> {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        for dependency in self {
            visit(dependency.id().untyped());
        }
    }
}

impl VisitAssetDependencies for Vec<UntypedHandle> {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        for dependency in self {
            visit(dependency.id());
        }
    }
}

/// Adds asset-related builder methods to [`App`].
pub trait AssetApp {
    /// Registers the given `loader` in the [`App`]'s [`AssetServer`].
    fn register_asset_loader<L: AssetLoader>(&mut self, loader: L) -> &mut Self;
    /// Registers the given `processor` in the [`App`]'s [`AssetProcessor`].
    fn register_asset_processor<P: Process>(&mut self, processor: P) -> &mut Self;
    /// Registers the given [`AssetSourceBuilder`] with the given `id`.
    ///
    /// Note that asset sources must be registered before adding [`AssetPlugin`] to your application,
    /// since registered asset sources are built at that point and not after.
    fn register_asset_source(
        &mut self,
        id: impl Into<AssetSourceId<'static>>,
        source: AssetSourceBuilder,
    ) -> &mut Self;
    /// Sets the default asset processor for the given `extension`.
    fn set_default_asset_processor<P: Process>(&mut self, extension: &str) -> &mut Self;
    /// Initializes the given loader in the [`App`]'s [`AssetServer`].
    fn init_asset_loader<L: AssetLoader + FromWorld>(&mut self) -> &mut Self;
    /// Initializes the given [`Asset`] in the [`App`] by:
    /// * Registering the [`Asset`] in the [`AssetServer`]
    /// * Initializing the [`AssetEvent`] resource for the [`Asset`]
    /// * Adding other relevant systems and resources for the [`Asset`]
    /// * Ignoring schedule ambiguities in [`Assets`] resource. Any time a system takes
    /// mutable access to this resource this causes a conflict, but they rarely actually
    /// modify the same underlying asset.
    fn init_asset<A: Asset>(&mut self) -> &mut Self;
    /// Registers the asset type `T` using `[App::register]`,
    /// and adds [`ReflectAsset`] type data to `T` and [`ReflectHandle`] type data to [`Handle<T>`] in the type registry.
    ///
    /// This enables reflection code to access assets. For detailed information, see the docs on [`ReflectAsset`] and [`ReflectHandle`].
    fn register_asset_reflect<A>(&mut self) -> &mut Self
    where
        A: Asset + Reflect + FromReflect + GetTypeRegistration;
    /// Preregisters a loader for the given extensions, that will block asset loads until a real loader
    /// is registered.
    fn preregister_asset_loader<L: AssetLoader>(&mut self, extensions: &[&str]) -> &mut Self;
}

impl AssetApp for App {
    fn register_asset_loader<L: AssetLoader>(&mut self, loader: L) -> &mut Self {
        self.world()
            .resource::<AssetServer>()
            .register_loader(loader);
        self
    }

    fn register_asset_processor<P: Process>(&mut self, processor: P) -> &mut Self {
        if let Some(asset_processor) = self.world().get_resource::<AssetProcessor>() {
            asset_processor.register_processor(processor);
        }
        self
    }

    fn register_asset_source(
        &mut self,
        id: impl Into<AssetSourceId<'static>>,
        source: AssetSourceBuilder,
    ) -> &mut Self {
        let id = id.into();
        if self.world().get_resource::<AssetServer>().is_some() {
            error!("{} must be registered before `AssetPlugin` (typically added as part of `DefaultPlugins`)", id);
        }

        {
            let mut sources = self
                .world_mut()
                .get_resource_or_insert_with(AssetSourceBuilders::default);
            sources.insert(id, source);
        }

        self
    }

    fn set_default_asset_processor<P: Process>(&mut self, extension: &str) -> &mut Self {
        if let Some(asset_processor) = self.world().get_resource::<AssetProcessor>() {
            asset_processor.set_default_processor::<P>(extension);
        }
        self
    }

    fn init_asset_loader<L: AssetLoader + FromWorld>(&mut self) -> &mut Self {
        let loader = L::from_world(self.world_mut());
        self.register_asset_loader(loader)
    }

    fn init_asset<A: Asset>(&mut self) -> &mut Self {
        let assets = Assets::<A>::default();
        self.world()
            .resource::<AssetServer>()
            .register_asset(&assets);
        if self.world().contains_resource::<AssetProcessor>() {
            let processor = self.world().resource::<AssetProcessor>();
            // The processor should have its own handle provider separate from the Asset storage
            // to ensure the id spaces are entirely separate. Not _strictly_ necessary, but
            // desirable.
            processor
                .server()
                .register_handle_provider(AssetHandleProvider::new(
                    TypeId::of::<A>(),
                    Arc::new(AssetIndexAllocator::default()),
                ));
        }
        self.insert_resource(assets)
            .allow_ambiguous_resource::<Assets<A>>()
            .add_event::<AssetEvent<A>>()
            .add_event::<AssetLoadFailedEvent<A>>()
            .register_type::<Handle<A>>()
            .add_systems(
                Last,
                Assets::<A>::asset_events
                    .run_if(Assets::<A>::asset_events_condition)
                    .in_set(AssetEvents),
            )
            .add_systems(PreUpdate, Assets::<A>::track_assets.in_set(TrackAssets))
    }

    fn register_asset_reflect<A>(&mut self) -> &mut Self
    where
        A: Asset + Reflect + FromReflect + GetTypeRegistration,
    {
        let type_registry = self.world().resource::<AppTypeRegistry>();
        {
            let mut type_registry = type_registry.write();

            type_registry.register::<A>();
            type_registry.register::<Handle<A>>();
            type_registry.register_type_data::<A, ReflectAsset>();
            type_registry.register_type_data::<Handle<A>, ReflectHandle>();
        }

        self
    }

    fn preregister_asset_loader<L: AssetLoader>(&mut self, extensions: &[&str]) -> &mut Self {
        self.world_mut()
            .resource_mut::<AssetServer>()
            .preregister_loader::<L>(extensions);
        self
    }
}

/// A system set that holds all "track asset" operations.
#[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone)]
pub struct TrackAssets;

/// A system set where events accumulated in [`Assets`] are applied to the [`AssetEvent`] [`Events`] resource.
///
/// [`Events`]: bevy_ecs::event::Events
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct AssetEvents;

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_asset,
        folder::LoadedFolder,
        handle::Handle,
        io::{
            gated::{GateOpener, GatedReader},
            memory::{Dir, MemoryAssetReader},
            AssetReader, AssetReaderError, AssetSource, AssetSourceId, Reader,
        },
        loader::{AssetLoader, LoadContext},
        Asset, AssetApp, AssetEvent, AssetId, AssetLoadError, AssetLoadFailedEvent, AssetPath,
        AssetPlugin, AssetServer, Assets, DependencyLoadState, LoadState,
        RecursiveDependencyLoadState,
    };
    use bevy_app::{App, Update};
    use bevy_core::TaskPoolPlugin;
    use bevy_ecs::prelude::*;
    use bevy_ecs::{
        event::ManualEventReader,
        schedule::{LogLevel, ScheduleBuildSettings},
    };
    use bevy_log::LogPlugin;
    use bevy_reflect::TypePath;
    use bevy_utils::{Duration, HashMap};
    use futures_lite::AsyncReadExt;
    use serde::{Deserialize, Serialize};
    use std::{path::Path, sync::Arc};
    use thiserror::Error;

    #[derive(Asset, TypePath, Debug, Default)]
    pub struct CoolText {
        pub text: String,
        pub embedded: String,
        #[dependency]
        pub dependencies: Vec<Handle<CoolText>>,
        #[dependency]
        pub sub_texts: Vec<Handle<SubText>>,
    }

    #[derive(Asset, TypePath, Debug)]
    pub struct SubText {
        text: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct CoolTextRon {
        text: String,
        dependencies: Vec<String>,
        embedded_dependencies: Vec<String>,
        sub_texts: Vec<String>,
    }

    #[derive(Default)]
    pub struct CoolTextLoader;

    #[derive(Error, Debug)]
    pub enum CoolTextLoaderError {
        #[error("Could not load dependency: {dependency}")]
        CannotLoadDependency { dependency: AssetPath<'static> },
        #[error("A RON error occurred during loading")]
        RonSpannedError(#[from] ron::error::SpannedError),
        #[error("An IO error occurred during loading")]
        Io(#[from] std::io::Error),
    }

    impl AssetLoader for CoolTextLoader {
        type Asset = CoolText;

        type Settings = ();

        type Error = CoolTextLoaderError;

        async fn load<'a>(
            &'a self,
            reader: &'a mut Reader<'_>,
            _settings: &'a Self::Settings,
            load_context: &'a mut LoadContext<'_>,
        ) -> Result<Self::Asset, Self::Error> {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut ron: CoolTextRon = ron::de::from_bytes(&bytes)?;
            let mut embedded = String::new();
            for dep in ron.embedded_dependencies {
                let loaded = load_context
                    .loader()
                    .direct()
                    .load::<CoolText>(&dep)
                    .await
                    .map_err(|_| Self::Error::CannotLoadDependency {
                        dependency: dep.into(),
                    })?;
                let cool = loaded.get();
                embedded.push_str(&cool.text);
            }
            Ok(CoolText {
                text: ron.text,
                embedded,
                dependencies: ron
                    .dependencies
                    .iter()
                    .map(|p| load_context.load(p))
                    .collect(),
                sub_texts: ron
                    .sub_texts
                    .drain(..)
                    .map(|text| load_context.add_labeled_asset(text.clone(), SubText { text }))
                    .collect(),
            })
        }

        fn extensions(&self) -> &[&str] {
            &["cool.ron"]
        }
    }

    /// A dummy [`CoolText`] asset reader that only succeeds after `failure_count` times it's read from for each asset.
    #[derive(Default, Clone)]
    pub struct UnstableMemoryAssetReader {
        pub attempt_counters: Arc<std::sync::Mutex<HashMap<Box<Path>, usize>>>,
        pub load_delay: Duration,
        memory_reader: MemoryAssetReader,
        failure_count: usize,
    }

    impl UnstableMemoryAssetReader {
        pub fn new(root: Dir, failure_count: usize) -> Self {
            Self {
                load_delay: Duration::from_millis(10),
                memory_reader: MemoryAssetReader { root },
                attempt_counters: Default::default(),
                failure_count,
            }
        }
    }

    impl AssetReader for UnstableMemoryAssetReader {
        async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
            self.memory_reader.is_directory(path).await
        }
        async fn read_directory<'a>(
            &'a self,
            path: &'a Path,
        ) -> Result<Box<bevy_asset::io::PathStream>, AssetReaderError> {
            self.memory_reader.read_directory(path).await
        }
        async fn read_meta<'a>(
            &'a self,
            path: &'a Path,
        ) -> Result<Box<bevy_asset::io::Reader<'a>>, AssetReaderError> {
            self.memory_reader.read_meta(path).await
        }
        async fn read<'a>(
            &'a self,
            path: &'a Path,
        ) -> Result<Box<bevy_asset::io::Reader<'a>>, bevy_asset::io::AssetReaderError> {
            let attempt_number = {
                let mut attempt_counters = self.attempt_counters.lock().unwrap();
                if let Some(existing) = attempt_counters.get_mut(path) {
                    *existing += 1;
                    *existing
                } else {
                    attempt_counters.insert(path.into(), 1);
                    1
                }
            };

            if attempt_number <= self.failure_count {
                let io_error = std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!(
                        "Simulated failure {attempt_number} of {}",
                        self.failure_count
                    ),
                );
                let wait = self.load_delay;
                return async move {
                    std::thread::sleep(wait);
                    Err(AssetReaderError::Io(io_error.into()))
                }
                .await;
            }

            self.memory_reader.read(path).await
        }
    }

    fn test_app(dir: Dir) -> (App, GateOpener) {
        let mut app = App::new();
        let (gated_memory_reader, gate_opener) = GatedReader::new(MemoryAssetReader { root: dir });
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSource::build().with_reader(move || Box::new(gated_memory_reader.clone())),
        )
        .add_plugins((
            TaskPoolPlugin::default(),
            LogPlugin::default(),
            AssetPlugin::default(),
        ));
        (app, gate_opener)
    }

    pub fn run_app_until(app: &mut App, mut predicate: impl FnMut(&mut World) -> Option<()>) {
        for _ in 0..LARGE_ITERATION_COUNT {
            app.update();
            if predicate(app.world_mut()).is_some() {
                return;
            }
        }

        panic!("Ran out of loops to return `Some` from `predicate`");
    }

    const LARGE_ITERATION_COUNT: usize = 10000;

    fn get<A: Asset>(world: &World, id: AssetId<A>) -> Option<&A> {
        world.resource::<Assets<A>>().get(id)
    }

    #[derive(Resource, Default)]
    struct StoredEvents(Vec<AssetEvent<CoolText>>);

    fn store_asset_events(
        mut reader: EventReader<AssetEvent<CoolText>>,
        mut storage: ResMut<StoredEvents>,
    ) {
        storage.0.extend(reader.read().cloned());
    }

    #[test]
    fn load_dependencies() {
        // The particular usage of GatedReader in this test will cause deadlocking if running single-threaded
        #[cfg(not(feature = "multi_threaded"))]
        panic!("This test requires the \"multi_threaded\" feature, otherwise it will deadlock.\ncargo test --package bevy_asset --features multi_threaded");

        let dir = Dir::default();

        let a_path = "a.cool.ron";
        let a_ron = r#"
(
    text: "a",
    dependencies: [
        "foo/b.cool.ron",
        "c.cool.ron",
    ],
    embedded_dependencies: [],
    sub_texts: [],
)"#;
        let b_path = "foo/b.cool.ron";
        let b_ron = r#"
(
    text: "b",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

        let c_path = "c.cool.ron";
        let c_ron = r#"
(
    text: "c",
    dependencies: [
        "d.cool.ron",
    ],
    embedded_dependencies: ["a.cool.ron", "foo/b.cool.ron"],
    sub_texts: ["hello"],
)"#;

        let d_path = "d.cool.ron";
        let d_ron = r#"
(
    text: "d",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

        dir.insert_asset_text(Path::new(a_path), a_ron);
        dir.insert_asset_text(Path::new(b_path), b_ron);
        dir.insert_asset_text(Path::new(c_path), c_ron);
        dir.insert_asset_text(Path::new(d_path), d_ron);

        #[derive(Resource)]
        struct IdResults {
            b_id: AssetId<CoolText>,
            c_id: AssetId<CoolText>,
            d_id: AssetId<CoolText>,
        }

        let (mut app, gate_opener) = test_app(dir);
        app.init_asset::<CoolText>()
            .init_asset::<SubText>()
            .init_resource::<StoredEvents>()
            .register_asset_loader(CoolTextLoader)
            .add_systems(Update, store_asset_events);
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle: Handle<CoolText> = asset_server.load(a_path);
        let a_id = handle.id();
        let entity = app.world_mut().spawn(handle).id();
        app.update();
        {
            let a_text = get::<CoolText>(app.world(), a_id);
            let (a_load, a_deps, a_rec_deps) = asset_server.get_load_states(a_id).unwrap();
            assert!(a_text.is_none(), "a's asset should not exist yet");
            assert_eq!(a_load, LoadState::Loading, "a should still be loading");
            assert_eq!(
                a_deps,
                DependencyLoadState::Loading,
                "a deps should still be loading"
            );
            assert_eq!(
                a_rec_deps,
                RecursiveDependencyLoadState::Loading,
                "a recursive deps should still be loading"
            );
        }

        // Allow "a" to load ... wait for it to finish loading and validate results
        // Dependencies are still gated so they should not be loaded yet
        gate_opener.open(a_path);
        run_app_until(&mut app, |world| {
            let a_text = get::<CoolText>(world, a_id)?;
            let (a_load, a_deps, a_rec_deps) = asset_server.get_load_states(a_id).unwrap();
            assert_eq!(a_text.text, "a");
            assert_eq!(a_text.dependencies.len(), 2);
            assert_eq!(a_load, LoadState::Loaded, "a is loaded");
            assert_eq!(a_deps, DependencyLoadState::Loading);
            assert_eq!(a_rec_deps, RecursiveDependencyLoadState::Loading);

            let b_id = a_text.dependencies[0].id();
            let b_text = get::<CoolText>(world, b_id);
            let (b_load, b_deps, b_rec_deps) = asset_server.get_load_states(b_id).unwrap();
            assert!(b_text.is_none(), "b component should not exist yet");
            assert_eq!(b_load, LoadState::Loading);
            assert_eq!(b_deps, DependencyLoadState::Loading);
            assert_eq!(b_rec_deps, RecursiveDependencyLoadState::Loading);

            let c_id = a_text.dependencies[1].id();
            let c_text = get::<CoolText>(world, c_id);
            let (c_load, c_deps, c_rec_deps) = asset_server.get_load_states(c_id).unwrap();
            assert!(c_text.is_none(), "c component should not exist yet");
            assert_eq!(c_load, LoadState::Loading);
            assert_eq!(c_deps, DependencyLoadState::Loading);
            assert_eq!(c_rec_deps, RecursiveDependencyLoadState::Loading);
            Some(())
        });

        // Allow "b" to load ... wait for it to finish loading and validate results
        // "c" should not be loaded yet
        gate_opener.open(b_path);
        run_app_until(&mut app, |world| {
            let a_text = get::<CoolText>(world, a_id)?;
            let (a_load, a_deps, a_rec_deps) = asset_server.get_load_states(a_id).unwrap();
            assert_eq!(a_text.text, "a");
            assert_eq!(a_text.dependencies.len(), 2);
            assert_eq!(a_load, LoadState::Loaded);
            assert_eq!(a_deps, DependencyLoadState::Loading);
            assert_eq!(a_rec_deps, RecursiveDependencyLoadState::Loading);

            let b_id = a_text.dependencies[0].id();
            let b_text = get::<CoolText>(world, b_id)?;
            let (b_load, b_deps, b_rec_deps) = asset_server.get_load_states(b_id).unwrap();
            assert_eq!(b_text.text, "b");
            assert_eq!(b_load, LoadState::Loaded);
            assert_eq!(b_deps, DependencyLoadState::Loaded);
            assert_eq!(b_rec_deps, RecursiveDependencyLoadState::Loaded);

            let c_id = a_text.dependencies[1].id();
            let c_text = get::<CoolText>(world, c_id);
            let (c_load, c_deps, c_rec_deps) = asset_server.get_load_states(c_id).unwrap();
            assert!(c_text.is_none(), "c component should not exist yet");
            assert_eq!(c_load, LoadState::Loading);
            assert_eq!(c_deps, DependencyLoadState::Loading);
            assert_eq!(c_rec_deps, RecursiveDependencyLoadState::Loading);
            Some(())
        });

        // Allow "c" to load ... wait for it to finish loading and validate results
        // all "a" dependencies should be loaded now
        gate_opener.open(c_path);

        // Re-open a and b gates to allow c to load embedded deps (gates are closed after each load)
        gate_opener.open(a_path);
        gate_opener.open(b_path);
        run_app_until(&mut app, |world| {
            let a_text = get::<CoolText>(world, a_id)?;
            let (a_load, a_deps, a_rec_deps) = asset_server.get_load_states(a_id).unwrap();
            assert_eq!(a_text.text, "a");
            assert_eq!(a_text.embedded, "");
            assert_eq!(a_text.dependencies.len(), 2);
            assert_eq!(a_load, LoadState::Loaded);

            let b_id = a_text.dependencies[0].id();
            let b_text = get::<CoolText>(world, b_id)?;
            let (b_load, b_deps, b_rec_deps) = asset_server.get_load_states(b_id).unwrap();
            assert_eq!(b_text.text, "b");
            assert_eq!(b_text.embedded, "");
            assert_eq!(b_load, LoadState::Loaded);
            assert_eq!(b_deps, DependencyLoadState::Loaded);
            assert_eq!(b_rec_deps, RecursiveDependencyLoadState::Loaded);

            let c_id = a_text.dependencies[1].id();
            let c_text = get::<CoolText>(world, c_id)?;
            let (c_load, c_deps, c_rec_deps) = asset_server.get_load_states(c_id).unwrap();
            assert_eq!(c_text.text, "c");
            assert_eq!(c_text.embedded, "ab");
            assert_eq!(c_load, LoadState::Loaded);
            assert_eq!(
                c_deps,
                DependencyLoadState::Loading,
                "c deps should not be loaded yet because d has not loaded"
            );
            assert_eq!(
                c_rec_deps,
                RecursiveDependencyLoadState::Loading,
                "c rec deps should not be loaded yet because d has not loaded"
            );

            let sub_text_id = c_text.sub_texts[0].id();
            let sub_text = get::<SubText>(world, sub_text_id)
                .expect("subtext should exist if c exists. it came from the same loader");
            assert_eq!(sub_text.text, "hello");
            let (sub_text_load, sub_text_deps, sub_text_rec_deps) =
                asset_server.get_load_states(sub_text_id).unwrap();
            assert_eq!(sub_text_load, LoadState::Loaded);
            assert_eq!(sub_text_deps, DependencyLoadState::Loaded);
            assert_eq!(sub_text_rec_deps, RecursiveDependencyLoadState::Loaded);

            let d_id = c_text.dependencies[0].id();
            let d_text = get::<CoolText>(world, d_id);
            let (d_load, d_deps, d_rec_deps) = asset_server.get_load_states(d_id).unwrap();
            assert!(d_text.is_none(), "d component should not exist yet");
            assert_eq!(d_load, LoadState::Loading);
            assert_eq!(d_deps, DependencyLoadState::Loading);
            assert_eq!(d_rec_deps, RecursiveDependencyLoadState::Loading);

            assert_eq!(
                a_deps,
                DependencyLoadState::Loaded,
                "If c has been loaded, the a deps should all be considered loaded"
            );
            assert_eq!(
                a_rec_deps,
                RecursiveDependencyLoadState::Loading,
                "d is not loaded, so a's recursive deps should still be loading"
            );
            world.insert_resource(IdResults { b_id, c_id, d_id });
            Some(())
        });

        gate_opener.open(d_path);
        run_app_until(&mut app, |world| {
            let a_text = get::<CoolText>(world, a_id)?;
            let (_a_load, _a_deps, a_rec_deps) = asset_server.get_load_states(a_id).unwrap();
            let c_id = a_text.dependencies[1].id();
            let c_text = get::<CoolText>(world, c_id)?;
            let (c_load, c_deps, c_rec_deps) = asset_server.get_load_states(c_id).unwrap();
            assert_eq!(c_text.text, "c");
            assert_eq!(c_text.embedded, "ab");

            let d_id = c_text.dependencies[0].id();
            let d_text = get::<CoolText>(world, d_id)?;
            let (d_load, d_deps, d_rec_deps) = asset_server.get_load_states(d_id).unwrap();
            assert_eq!(d_text.text, "d");
            assert_eq!(d_text.embedded, "");

            assert_eq!(c_load, LoadState::Loaded);
            assert_eq!(c_deps, DependencyLoadState::Loaded);
            assert_eq!(c_rec_deps, RecursiveDependencyLoadState::Loaded);

            assert_eq!(d_load, LoadState::Loaded);
            assert_eq!(d_deps, DependencyLoadState::Loaded);
            assert_eq!(d_rec_deps, RecursiveDependencyLoadState::Loaded);

            assert_eq!(
                a_rec_deps,
                RecursiveDependencyLoadState::Loaded,
                "d is loaded, so a's recursive deps should be loaded"
            );
            Some(())
        });

        {
            let mut texts = app.world_mut().resource_mut::<Assets<CoolText>>();
            let a = texts.get_mut(a_id).unwrap();
            a.text = "Changed".to_string();
        }

        app.world_mut().despawn(entity);
        app.update();
        assert_eq!(
            app.world().resource::<Assets<CoolText>>().len(),
            0,
            "CoolText asset entities should be despawned when no more handles exist"
        );
        app.update();
        // this requires a second update because the parent asset was freed in the previous app.update()
        assert_eq!(
            app.world().resource::<Assets<SubText>>().len(),
            0,
            "SubText asset entities should be despawned when no more handles exist"
        );
        let events = app.world_mut().remove_resource::<StoredEvents>().unwrap();
        let id_results = app.world_mut().remove_resource::<IdResults>().unwrap();
        let expected_events = vec![
            AssetEvent::Added { id: a_id },
            AssetEvent::LoadedWithDependencies {
                id: id_results.b_id,
            },
            AssetEvent::Added {
                id: id_results.b_id,
            },
            AssetEvent::Added {
                id: id_results.c_id,
            },
            AssetEvent::LoadedWithDependencies {
                id: id_results.d_id,
            },
            AssetEvent::LoadedWithDependencies {
                id: id_results.c_id,
            },
            AssetEvent::LoadedWithDependencies { id: a_id },
            AssetEvent::Added {
                id: id_results.d_id,
            },
            AssetEvent::Modified { id: a_id },
            AssetEvent::Unused { id: a_id },
            AssetEvent::Removed { id: a_id },
            AssetEvent::Unused {
                id: id_results.b_id,
            },
            AssetEvent::Removed {
                id: id_results.b_id,
            },
            AssetEvent::Unused {
                id: id_results.c_id,
            },
            AssetEvent::Removed {
                id: id_results.c_id,
            },
            AssetEvent::Unused {
                id: id_results.d_id,
            },
            AssetEvent::Removed {
                id: id_results.d_id,
            },
        ];
        assert_eq!(events.0, expected_events);
    }

    #[test]
    fn failure_load_states() {
        // The particular usage of GatedReader in this test will cause deadlocking if running single-threaded
        #[cfg(not(feature = "multi_threaded"))]
        panic!("This test requires the \"multi_threaded\" feature, otherwise it will deadlock.\ncargo test --package bevy_asset --features multi_threaded");

        let dir = Dir::default();

        let a_path = "a.cool.ron";
        let a_ron = r#"
(
    text: "a",
    dependencies: [
        "b.cool.ron",
        "c.cool.ron",
    ],
    embedded_dependencies: [],
    sub_texts: []
)"#;
        let b_path = "b.cool.ron";
        let b_ron = r#"
(
    text: "b",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: []
)"#;

        let c_path = "c.cool.ron";
        let c_ron = r#"
(
    text: "c",
    dependencies: [
        "d.cool.ron",
    ],
    embedded_dependencies: [],
    sub_texts: []
)"#;

        let d_path = "d.cool.ron";
        let d_ron = r#"
(
    text: "d",
    dependencies: [],
    OH NO THIS ASSET IS MALFORMED
    embedded_dependencies: [],
    sub_texts: []
)"#;

        dir.insert_asset_text(Path::new(a_path), a_ron);
        dir.insert_asset_text(Path::new(b_path), b_ron);
        dir.insert_asset_text(Path::new(c_path), c_ron);
        dir.insert_asset_text(Path::new(d_path), d_ron);

        let (mut app, gate_opener) = test_app(dir);
        app.init_asset::<CoolText>()
            .register_asset_loader(CoolTextLoader);
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle: Handle<CoolText> = asset_server.load(a_path);
        let a_id = handle.id();
        {
            let other_handle: Handle<CoolText> = asset_server.load(a_path);
            assert_eq!(
                other_handle, handle,
                "handles from consecutive load calls should be equal"
            );
            assert_eq!(
                other_handle.id(),
                handle.id(),
                "handle ids from consecutive load calls should be equal"
            );
        }

        app.world_mut().spawn(handle);
        gate_opener.open(a_path);
        gate_opener.open(b_path);
        gate_opener.open(c_path);
        gate_opener.open(d_path);

        run_app_until(&mut app, |world| {
            let a_text = get::<CoolText>(world, a_id)?;
            let (a_load, a_deps, a_rec_deps) = asset_server.get_load_states(a_id).unwrap();

            let b_id = a_text.dependencies[0].id();
            let b_text = get::<CoolText>(world, b_id)?;
            let (b_load, b_deps, b_rec_deps) = asset_server.get_load_states(b_id).unwrap();

            let c_id = a_text.dependencies[1].id();
            let c_text = get::<CoolText>(world, c_id)?;
            let (c_load, c_deps, c_rec_deps) = asset_server.get_load_states(c_id).unwrap();

            let d_id = c_text.dependencies[0].id();
            let d_text = get::<CoolText>(world, d_id);
            let (d_load, d_deps, d_rec_deps) = asset_server.get_load_states(d_id).unwrap();
            if !matches!(d_load, LoadState::Failed(_)) {
                // wait until d has exited the loading state
                return None;
            }

            assert!(d_text.is_none());
            assert!(matches!(d_load, LoadState::Failed(_)));
            assert_eq!(d_deps, DependencyLoadState::Failed);
            assert_eq!(d_rec_deps, RecursiveDependencyLoadState::Failed);

            assert_eq!(a_text.text, "a");
            assert_eq!(a_load, LoadState::Loaded);
            assert_eq!(a_deps, DependencyLoadState::Loaded);
            assert_eq!(a_rec_deps, RecursiveDependencyLoadState::Failed);

            assert_eq!(b_text.text, "b");
            assert_eq!(b_load, LoadState::Loaded);
            assert_eq!(b_deps, DependencyLoadState::Loaded);
            assert_eq!(b_rec_deps, RecursiveDependencyLoadState::Loaded);

            assert_eq!(c_text.text, "c");
            assert_eq!(c_load, LoadState::Loaded);
            assert_eq!(c_deps, DependencyLoadState::Failed);
            assert_eq!(c_rec_deps, RecursiveDependencyLoadState::Failed);

            Some(())
        });
    }

    const SIMPLE_TEXT: &str = r#"
(
    text: "dep",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;
    #[test]
    fn keep_gotten_strong_handles() {
        let dir = Dir::default();
        dir.insert_asset_text(Path::new("dep.cool.ron"), SIMPLE_TEXT);

        let (mut app, _) = test_app(dir);
        app.init_asset::<CoolText>()
            .init_asset::<SubText>()
            .init_resource::<StoredEvents>()
            .register_asset_loader(CoolTextLoader)
            .add_systems(Update, store_asset_events);

        let id = {
            let handle = {
                let mut texts = app.world_mut().resource_mut::<Assets<CoolText>>();
                let handle = texts.add(CoolText::default());
                texts.get_strong_handle(handle.id()).unwrap()
            };

            app.update();

            {
                let text = app.world().resource::<Assets<CoolText>>().get(&handle);
                assert!(text.is_some());
            }
            handle.id()
        };
        // handle is dropped
        app.update();
        assert!(
            app.world().resource::<Assets<CoolText>>().get(id).is_none(),
            "asset has no handles, so it should have been dropped last update"
        );
    }

    #[test]
    fn manual_asset_management() {
        // The particular usage of GatedReader in this test will cause deadlocking if running single-threaded
        #[cfg(not(feature = "multi_threaded"))]
        panic!("This test requires the \"multi_threaded\" feature, otherwise it will deadlock.\ncargo test --package bevy_asset --features multi_threaded");

        let dir = Dir::default();
        let dep_path = "dep.cool.ron";

        dir.insert_asset_text(Path::new(dep_path), SIMPLE_TEXT);

        let (mut app, gate_opener) = test_app(dir);
        app.init_asset::<CoolText>()
            .init_asset::<SubText>()
            .init_resource::<StoredEvents>()
            .register_asset_loader(CoolTextLoader)
            .add_systems(Update, store_asset_events);

        let hello = "hello".to_string();
        let empty = "".to_string();

        let id = {
            let handle = {
                let mut texts = app.world_mut().resource_mut::<Assets<CoolText>>();
                texts.add(CoolText {
                    text: hello.clone(),
                    embedded: empty.clone(),
                    dependencies: vec![],
                    sub_texts: Vec::new(),
                })
            };

            app.update();

            {
                let text = app
                    .world()
                    .resource::<Assets<CoolText>>()
                    .get(&handle)
                    .unwrap();
                assert_eq!(text.text, hello);
            }
            handle.id()
        };
        // handle is dropped
        app.update();
        assert!(
            app.world().resource::<Assets<CoolText>>().get(id).is_none(),
            "asset has no handles, so it should have been dropped last update"
        );
        // remove event is emitted
        app.update();
        let events = std::mem::take(&mut app.world_mut().resource_mut::<StoredEvents>().0);
        let expected_events = vec![
            AssetEvent::Added { id },
            AssetEvent::Unused { id },
            AssetEvent::Removed { id },
        ];
        assert_eq!(events, expected_events);

        let dep_handle = app.world().resource::<AssetServer>().load(dep_path);
        let a = CoolText {
            text: "a".to_string(),
            embedded: empty,
            // this dependency is behind a manual load gate, which should prevent 'a' from emitting a LoadedWithDependencies event
            dependencies: vec![dep_handle.clone()],
            sub_texts: Vec::new(),
        };
        let a_handle = app.world().resource::<AssetServer>().load_asset(a);
        app.update();
        // TODO: ideally it doesn't take two updates for the added event to emit
        app.update();

        let events = std::mem::take(&mut app.world_mut().resource_mut::<StoredEvents>().0);
        let expected_events = vec![AssetEvent::Added { id: a_handle.id() }];
        assert_eq!(events, expected_events);

        gate_opener.open(dep_path);
        loop {
            app.update();
            let events = std::mem::take(&mut app.world_mut().resource_mut::<StoredEvents>().0);
            if events.is_empty() {
                continue;
            }
            let expected_events = vec![
                AssetEvent::LoadedWithDependencies {
                    id: dep_handle.id(),
                },
                AssetEvent::LoadedWithDependencies { id: a_handle.id() },
            ];
            assert_eq!(events, expected_events);
            break;
        }
        app.update();
        let events = std::mem::take(&mut app.world_mut().resource_mut::<StoredEvents>().0);
        let expected_events = vec![AssetEvent::Added {
            id: dep_handle.id(),
        }];
        assert_eq!(events, expected_events);
    }

    #[test]
    fn load_folder() {
        // The particular usage of GatedReader in this test will cause deadlocking if running single-threaded
        #[cfg(not(feature = "multi_threaded"))]
        panic!("This test requires the \"multi_threaded\" feature, otherwise it will deadlock.\ncargo test --package bevy_asset --features multi_threaded");

        let dir = Dir::default();

        let a_path = "text/a.cool.ron";
        let a_ron = r#"
(
    text: "a",
    dependencies: [
        "b.cool.ron",
    ],
    embedded_dependencies: [],
    sub_texts: [],
)"#;
        let b_path = "b.cool.ron";
        let b_ron = r#"
(
    text: "b",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

        let c_path = "text/c.cool.ron";
        let c_ron = r#"
(
    text: "c",
    dependencies: [
    ],
    embedded_dependencies: [],
    sub_texts: [],
)"#;
        dir.insert_asset_text(Path::new(a_path), a_ron);
        dir.insert_asset_text(Path::new(b_path), b_ron);
        dir.insert_asset_text(Path::new(c_path), c_ron);

        let (mut app, gate_opener) = test_app(dir);
        app.init_asset::<CoolText>()
            .init_asset::<SubText>()
            .register_asset_loader(CoolTextLoader);
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle: Handle<LoadedFolder> = asset_server.load_folder("text");
        gate_opener.open(a_path);
        gate_opener.open(b_path);
        gate_opener.open(c_path);

        let mut reader = ManualEventReader::default();
        run_app_until(&mut app, |world| {
            let events = world.resource::<Events<AssetEvent<LoadedFolder>>>();
            let asset_server = world.resource::<AssetServer>();
            let loaded_folders = world.resource::<Assets<LoadedFolder>>();
            let cool_texts = world.resource::<Assets<CoolText>>();
            for event in reader.read(events) {
                if let AssetEvent::LoadedWithDependencies { id } = event {
                    if *id == handle.id() {
                        let loaded_folder = loaded_folders.get(&handle).unwrap();
                        let a_handle: Handle<CoolText> =
                            asset_server.get_handle("text/a.cool.ron").unwrap();
                        let c_handle: Handle<CoolText> =
                            asset_server.get_handle("text/c.cool.ron").unwrap();

                        let mut found_a = false;
                        let mut found_c = false;
                        for asset_handle in &loaded_folder.handles {
                            if asset_handle.id() == a_handle.id().untyped() {
                                found_a = true;
                            } else if asset_handle.id() == c_handle.id().untyped() {
                                found_c = true;
                            }
                        }
                        assert!(found_a);
                        assert!(found_c);
                        assert_eq!(loaded_folder.handles.len(), 2);

                        let a_text = cool_texts.get(&a_handle).unwrap();
                        let b_text = cool_texts.get(&a_text.dependencies[0]).unwrap();
                        let c_text = cool_texts.get(&c_handle).unwrap();

                        assert_eq!("a", a_text.text);
                        assert_eq!("b", b_text.text);
                        assert_eq!("c", c_text.text);

                        return Some(());
                    }
                }
            }
            None
        });
    }

    /// Tests that `AssetLoadFailedEvent<A>` events are emitted and can be used to retry failed assets.
    #[test]
    fn load_error_events() {
        #[derive(Resource, Default)]
        struct ErrorTracker {
            tick: u64,
            failures: usize,
            queued_retries: Vec<(AssetPath<'static>, AssetId<CoolText>, u64)>,
            finished_asset: Option<AssetId<CoolText>>,
        }

        fn asset_event_handler(
            mut events: EventReader<AssetEvent<CoolText>>,
            mut tracker: ResMut<ErrorTracker>,
        ) {
            for event in events.read() {
                if let AssetEvent::LoadedWithDependencies { id } = event {
                    tracker.finished_asset = Some(*id);
                }
            }
        }

        fn asset_load_error_event_handler(
            server: Res<AssetServer>,
            mut errors: EventReader<AssetLoadFailedEvent<CoolText>>,
            mut tracker: ResMut<ErrorTracker>,
        ) {
            // In the real world, this would refer to time (not ticks)
            tracker.tick += 1;

            // Retry loading past failed items
            let now = tracker.tick;
            tracker
                .queued_retries
                .retain(|(path, old_id, retry_after)| {
                    if now > *retry_after {
                        let new_handle = server.load::<CoolText>(path);
                        assert_eq!(&new_handle.id(), old_id);
                        false
                    } else {
                        true
                    }
                });

            // Check what just failed
            for error in errors.read() {
                let (load_state, _, _) = server.get_load_states(error.id).unwrap();
                assert!(matches!(load_state, LoadState::Failed(_)));
                assert_eq!(*error.path.source(), AssetSourceId::Name("unstable".into()));
                match &error.error {
                    AssetLoadError::AssetReaderError(read_error) => match read_error {
                        AssetReaderError::Io(_) => {
                            tracker.failures += 1;
                            if tracker.failures <= 2 {
                                // Retry in 10 ticks
                                tracker.queued_retries.push((
                                    error.path.clone(),
                                    error.id,
                                    now + 10,
                                ));
                            } else {
                                panic!(
                                    "Unexpected failure #{} (expected only 2)",
                                    tracker.failures
                                );
                            }
                        }
                        _ => panic!("Unexpected error type {:?}", read_error),
                    },
                    _ => panic!("Unexpected error type {:?}", error.error),
                }
            }
        }

        let a_path = "text/a.cool.ron";
        let a_ron = r#"
(
    text: "a",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

        let dir = Dir::default();
        dir.insert_asset_text(Path::new(a_path), a_ron);
        let unstable_reader = UnstableMemoryAssetReader::new(dir, 2);

        let mut app = App::new();
        app.register_asset_source(
            "unstable",
            AssetSource::build().with_reader(move || Box::new(unstable_reader.clone())),
        )
        .add_plugins((
            TaskPoolPlugin::default(),
            LogPlugin::default(),
            AssetPlugin::default(),
        ))
        .init_asset::<CoolText>()
        .register_asset_loader(CoolTextLoader)
        .init_resource::<ErrorTracker>()
        .add_systems(
            Update,
            (asset_event_handler, asset_load_error_event_handler).chain(),
        );

        let asset_server = app.world().resource::<AssetServer>().clone();
        let a_path = format!("unstable://{a_path}");
        let a_handle: Handle<CoolText> = asset_server.load(a_path);
        let a_id = a_handle.id();

        app.world_mut().spawn(a_handle);

        run_app_until(&mut app, |world| {
            let tracker = world.resource::<ErrorTracker>();
            match tracker.finished_asset {
                Some(asset_id) => {
                    assert_eq!(asset_id, a_id);
                    let assets = world.resource::<Assets<CoolText>>();
                    let result = assets.get(asset_id).unwrap();
                    assert_eq!(result.text, "a");
                    Some(())
                }
                None => None,
            }
        });
    }

    #[test]
    fn ignore_system_ambiguities_on_assets() {
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default())
            .init_asset::<CoolText>();

        fn uses_assets(_asset: ResMut<Assets<CoolText>>) {}
        app.add_systems(Update, (uses_assets, uses_assets));
        app.edit_schedule(Update, |s| {
            s.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Error,
                ..Default::default()
            });
        });

        // running schedule does not error on ambiguity between the 2 uses_assets systems
        app.world_mut().run_schedule(Update);
    }

    // validate the Asset derive macro for various asset types
    #[derive(Asset, TypePath)]
    pub struct TestAsset;

    #[allow(dead_code)]
    #[derive(Asset, TypePath)]
    pub enum EnumTestAsset {
        Unnamed(#[dependency] Handle<TestAsset>),
        Named {
            #[dependency]
            handle: Handle<TestAsset>,
            #[dependency]
            vec_handles: Vec<Handle<TestAsset>>,
            #[dependency]
            embedded: TestAsset,
        },
        StructStyle(#[dependency] TestAsset),
        Empty,
    }

    #[allow(dead_code)]
    #[derive(Asset, TypePath)]
    pub struct StructTestAsset {
        #[dependency]
        handle: Handle<TestAsset>,
        #[dependency]
        embedded: TestAsset,
    }

    #[allow(dead_code)]
    #[derive(Asset, TypePath)]
    pub struct TupleTestAsset(#[dependency] Handle<TestAsset>);
}
