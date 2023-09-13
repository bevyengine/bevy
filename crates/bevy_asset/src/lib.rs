#![allow(clippy::type_complexity)]

pub mod io;
pub mod meta;
pub mod processor;
pub mod saver;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        Asset, AssetApp, AssetEvent, AssetId, AssetPlugin, AssetServer, Assets, Handle,
        UntypedHandle,
    };
}

mod assets;
mod event;
mod folder;
mod handle;
mod id;
mod loader;
mod path;
mod reflect;
mod server;

pub use assets::*;
pub use bevy_asset_macros::Asset;
pub use event::*;
pub use folder::*;
pub use futures_lite::{AsyncReadExt, AsyncWriteExt};
pub use handle::*;
pub use id::*;
pub use loader::*;
pub use path::*;
pub use reflect::*;
pub use server::*;

pub use anyhow;
pub use bevy_utils::BoxedFuture;

use crate::{
    io::{processor_gated::ProcessorGatedReader, AssetProvider, AssetProviders},
    processor::{AssetProcessor, Process},
};
use bevy_app::{App, First, MainScheduleOrder, Plugin, PostUpdate, Startup};
use bevy_ecs::{
    reflect::AppTypeRegistry,
    schedule::{IntoSystemConfigs, IntoSystemSetConfigs, ScheduleLabel, SystemSet},
    world::FromWorld,
};
use bevy_reflect::{FromReflect, GetTypeRegistration, Reflect, TypePath};
use std::{any::TypeId, sync::Arc};

/// Provides "asset" loading and processing functionality. An [`Asset`] is a "runtime value" that is loaded from an [`AssetProvider`],
/// which can be something like a filesystem, a network, etc.
///
/// Supports flexible "modes", such as [`AssetPlugin::Processed`] and
/// [`AssetPlugin::Unprocessed`] that enable using the asset workflow that best suits your project.
pub enum AssetPlugin {
    /// Loads assets without any "preprocessing" from the configured asset `source` (defaults to the `assets` folder).
    Unprocessed {
        source: AssetProvider,
        watch_for_changes: bool,
    },
    /// Loads "processed" assets from a given `destination` source (defaults to the `imported_assets/Default` folder). This should
    /// generally only be used when distributing apps. Use [`AssetPlugin::ProcessedDev`] to develop apps that process assets,
    /// then switch to [`AssetPlugin::Processed`] when deploying the apps.
    Processed {
        destination: AssetProvider,
        watch_for_changes: bool,
    },
    /// Starts an [`AssetProcessor`] in the background that reads assets from the `source` provider (defaults to the `assets` folder),
    /// processes them according to their [`AssetMeta`], and writes them to the `destination` provider (defaults to the `imported_assets/Default` folder).
    ///
    /// By default this will hot reload changes to the `source` provider, resulting in reprocessing the asset and reloading it in the [`App`].
    ///
    /// [`AssetMeta`]: crate::meta::AssetMeta
    ProcessedDev {
        source: AssetProvider,
        destination: AssetProvider,
        watch_for_changes: bool,
    },
}

impl Default for AssetPlugin {
    fn default() -> Self {
        Self::unprocessed()
    }
}

impl AssetPlugin {
    const DEFAULT_FILE_SOURCE: &'static str = "assets";
    /// NOTE: this is in the Default sub-folder to make this forward compatible with "import profiles"
    /// and to allow us to put the "processor transaction log" at `imported_assets/log`
    const DEFAULT_FILE_DESTINATION: &'static str = "imported_assets/Default";

    /// Returns the default [`AssetPlugin::Processed`] configuration
    pub fn processed() -> Self {
        Self::Processed {
            destination: Default::default(),
            watch_for_changes: false,
        }
    }

    /// Returns the default [`AssetPlugin::ProcessedDev`] configuration
    pub fn processed_dev() -> Self {
        Self::ProcessedDev {
            source: Default::default(),
            destination: Default::default(),
            watch_for_changes: true,
        }
    }

    /// Returns the default [`AssetPlugin::Unprocessed`] configuration
    pub fn unprocessed() -> Self {
        Self::Unprocessed {
            source: Default::default(),
            watch_for_changes: false,
        }
    }

    /// Enables watching for changes, which will hot-reload assets when they change.
    pub fn watch_for_changes(mut self) -> Self {
        match &mut self {
            AssetPlugin::Unprocessed {
                watch_for_changes, ..
            }
            | AssetPlugin::Processed {
                watch_for_changes, ..
            }
            | AssetPlugin::ProcessedDev {
                watch_for_changes, ..
            } => *watch_for_changes = true,
        };
        self
    }
}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(UpdateAssets)
            .init_schedule(AssetEvents)
            .init_resource::<AssetProviders>();
        {
            match self {
                AssetPlugin::Unprocessed {
                    source,
                    watch_for_changes,
                } => {
                    let source_reader = app
                        .world
                        .resource_mut::<AssetProviders>()
                        .get_source_reader(source);
                    app.insert_resource(AssetServer::new(source_reader, *watch_for_changes));
                }
                AssetPlugin::Processed {
                    destination,
                    watch_for_changes,
                } => {
                    let destination_reader = app
                        .world
                        .resource_mut::<AssetProviders>()
                        .get_destination_reader(destination);
                    app.insert_resource(AssetServer::new(destination_reader, *watch_for_changes));
                }
                AssetPlugin::ProcessedDev {
                    source,
                    destination,
                    watch_for_changes,
                } => {
                    let mut asset_providers = app.world.resource_mut::<AssetProviders>();
                    let processor = AssetProcessor::new(&mut asset_providers, source, destination);
                    let destination_reader = asset_providers.get_destination_reader(source);
                    // the main asset server gates loads based on asset state
                    let gated_reader =
                        ProcessorGatedReader::new(destination_reader, processor.data.clone());
                    // the main asset server shares loaders with the processor asset server
                    app.insert_resource(AssetServer::new_with_loaders(
                        Box::new(gated_reader),
                        processor.server().data.loaders.clone(),
                        *watch_for_changes,
                    ))
                    .insert_resource(processor)
                    .add_systems(Startup, AssetProcessor::start);
                }
            }
        }
        app.init_asset::<LoadedFolder>()
            .init_asset::<()>()
            .configure_sets(
                UpdateAssets,
                TrackAssets.after(server::handle_internal_asset_events),
            )
            .add_systems(UpdateAssets, server::handle_internal_asset_events);

        let mut order = app.world.resource_mut::<MainScheduleOrder>();
        order.insert_after(First, UpdateAssets);
        order.insert_after(PostUpdate, AssetEvents);
    }
}

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
    /// Sets the default asset processor for the given `extension`.
    fn set_default_asset_processor<P: Process>(&mut self, extension: &str) -> &mut Self;
    /// Initializes the given loader in the [`App`]'s [`AssetServer`].
    fn init_asset_loader<L: AssetLoader + FromWorld>(&mut self) -> &mut Self;
    /// Initializes the given [`Asset`] in the [`App`] by:
    /// * Registering the [`Asset`] in the [`AssetServer`]
    /// * Initializing the [`AssetEvent`] resource for the [`Asset`]
    /// * Adding other relevant systems and resources for the [`Asset`]
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
        self.world.resource::<AssetServer>().register_loader(loader);
        self
    }

    fn init_asset_loader<L: AssetLoader + FromWorld>(&mut self) -> &mut Self {
        let loader = L::from_world(&mut self.world);
        self.register_asset_loader(loader)
    }

    fn init_asset<A: Asset>(&mut self) -> &mut Self {
        let assets = Assets::<A>::default();
        self.world.resource::<AssetServer>().register_asset(&assets);
        if self.world.contains_resource::<AssetProcessor>() {
            let processor = self.world.resource::<AssetProcessor>();
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
            .add_event::<AssetEvent<A>>()
            .register_type::<Handle<A>>()
            .register_type::<AssetId<A>>()
            .add_systems(AssetEvents, Assets::<A>::asset_events)
            .add_systems(UpdateAssets, Assets::<A>::track_assets.in_set(TrackAssets))
    }

    fn register_asset_reflect<A>(&mut self) -> &mut Self
    where
        A: Asset + Reflect + FromReflect + GetTypeRegistration,
    {
        let type_registry = self.world.resource::<AppTypeRegistry>();
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
        self.world
            .resource_mut::<AssetServer>()
            .preregister_loader::<L>(extensions);
        self
    }

    fn register_asset_processor<P: Process>(&mut self, processor: P) -> &mut Self {
        if let Some(asset_processor) = self.world.get_resource::<AssetProcessor>() {
            asset_processor.register_processor(processor);
        }
        self
    }

    fn set_default_asset_processor<P: Process>(&mut self, extension: &str) -> &mut Self {
        if let Some(asset_processor) = self.world.get_resource::<AssetProcessor>() {
            asset_processor.set_default_processor::<P>(extension);
        }
        self
    }
}

/// A system set that holds all "track asset" operations.
#[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone)]
pub struct TrackAssets;

/// Schedule where [`Assets`] resources are updated.
#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
pub struct UpdateAssets;

/// Schedule where events accumulated in [`Assets`] are applied to the [`AssetEvent`] [`Events`] resource.
///
/// [`Events`]: bevy_ecs::event::Events
#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
pub struct AssetEvents;

/// Loads an "internal" asset by embedding the string stored in the given `path_str` and associates it with the given handle.
#[macro_export]
macro_rules! load_internal_asset {
    ($app: ident, $handle: expr, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.insert($handle, ($loader)(
            include_str!($path_str),
            std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join($path_str)
                .to_string_lossy()
        ));
    }};
    // we can't support params without variadic arguments, so internal assets with additional params can't be hot-reloaded
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr $(, $param:expr)+) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.insert($handle, ($loader)(
            include_str!($path_str),
            std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join($path_str)
                .to_string_lossy(),
            $($param),+
        ));
    }};
}

/// Loads an "internal" binary asset by embedding the bytes stored in the given `path_str` and associates it with the given handle.
#[macro_export]
macro_rules! load_internal_binary_asset {
    ($app: ident, $handle: expr, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.insert(
            $handle,
            ($loader)(
                include_bytes!($path_str).as_ref(),
                std::path::Path::new(file!())
                    .parent()
                    .unwrap()
                    .join($path_str)
                    .to_string_lossy()
                    .into(),
            ),
        );
    }};
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_asset,
        folder::LoadedFolder,
        handle::Handle,
        io::{
            gated::{GateOpener, GatedReader},
            memory::{Dir, MemoryAssetReader},
            Reader,
        },
        loader::{AssetLoader, LoadContext},
        Asset, AssetApp, AssetEvent, AssetId, AssetPlugin, AssetProvider, AssetProviders,
        AssetServer, Assets, DependencyLoadState, LoadState, RecursiveDependencyLoadState,
    };
    use bevy_app::{App, Update};
    use bevy_core::TaskPoolPlugin;
    use bevy_ecs::event::ManualEventReader;
    use bevy_ecs::prelude::*;
    use bevy_log::LogPlugin;
    use bevy_reflect::TypePath;
    use bevy_utils::BoxedFuture;
    use futures_lite::AsyncReadExt;
    use serde::{Deserialize, Serialize};
    use std::path::Path;

    #[derive(Asset, TypePath, Debug)]
    pub struct CoolText {
        text: String,
        embedded: String,
        #[dependency]
        dependencies: Vec<Handle<CoolText>>,
        #[dependency]
        sub_texts: Vec<Handle<SubText>>,
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
    struct CoolTextLoader;

    impl AssetLoader for CoolTextLoader {
        type Asset = CoolText;

        type Settings = ();

        fn load<'a>(
            &'a self,
            reader: &'a mut Reader,
            _settings: &'a Self::Settings,
            load_context: &'a mut LoadContext,
        ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
            Box::pin(async move {
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).await?;
                let mut ron: CoolTextRon = ron::de::from_bytes(&bytes)?;
                let mut embedded = String::new();
                for dep in ron.embedded_dependencies {
                    let loaded = load_context.load_direct(&dep).await?;
                    let cool = loaded.get::<CoolText>().unwrap();
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
            })
        }

        fn extensions(&self) -> &[&str] {
            &["cool.ron"]
        }
    }

    fn test_app(dir: Dir) -> (App, GateOpener) {
        let mut app = App::new();
        let (gated_memory_reader, gate_opener) = GatedReader::new(MemoryAssetReader { root: dir });
        app.insert_resource(
            AssetProviders::default()
                .with_reader("Test", move || Box::new(gated_memory_reader.clone())),
        )
        .add_plugins((
            TaskPoolPlugin::default(),
            LogPlugin::default(),
            AssetPlugin::Unprocessed {
                source: AssetProvider::Custom("Test".to_string()),
                watch_for_changes: false,
            },
        ));
        (app, gate_opener)
    }

    fn run_app_until(app: &mut App, mut predicate: impl FnMut(&mut World) -> Option<()>) {
        for _ in 0..LARGE_ITERATION_COUNT {
            app.update();
            if (predicate)(&mut app.world).is_some() {
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
        let asset_server = app.world.resource::<AssetServer>().clone();
        let handle: Handle<CoolText> = asset_server.load(a_path);
        let a_id = handle.id();
        let entity = app.world.spawn(handle).id();
        app.update();
        {
            let a_text = get::<CoolText>(&app.world, a_id);
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
            let mut texts = app.world.resource_mut::<Assets<CoolText>>();
            let a = texts.get_mut(a_id).unwrap();
            a.text = "Changed".to_string();
        }

        app.world.despawn(entity);
        app.update();
        assert_eq!(
            app.world.resource::<Assets<CoolText>>().len(),
            0,
            "CoolText asset entities should be despawned when no more handles exist"
        );
        app.update();
        // this requires a second update because the parent asset was freed in the previous app.update()
        assert_eq!(
            app.world.resource::<Assets<SubText>>().len(),
            0,
            "SubText asset entities should be despawned when no more handles exist"
        );
        let events = app.world.remove_resource::<StoredEvents>().unwrap();
        let id_results = app.world.remove_resource::<IdResults>().unwrap();
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
            AssetEvent::Removed { id: a_id },
            AssetEvent::Removed {
                id: id_results.b_id,
            },
            AssetEvent::Removed {
                id: id_results.c_id,
            },
            AssetEvent::Removed {
                id: id_results.d_id,
            },
        ];
        assert_eq!(events.0, expected_events);
    }

    #[test]
    fn failure_load_states() {
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
        let asset_server = app.world.resource::<AssetServer>().clone();
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

        app.world.spawn(handle);
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
            if d_load != LoadState::Failed {
                // wait until d has exited the loading state
                return None;
            }

            assert!(d_text.is_none());
            assert_eq!(d_load, LoadState::Failed);
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

    #[test]
    fn manual_asset_management() {
        let dir = Dir::default();

        let dep_path = "dep.cool.ron";
        let dep_ron = r#"
(
    text: "dep",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

        dir.insert_asset_text(Path::new(dep_path), dep_ron);

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
                let mut texts = app.world.resource_mut::<Assets<CoolText>>();
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
                    .world
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
            app.world.resource::<Assets<CoolText>>().get(id).is_none(),
            "asset has no handles, so it should have been dropped last update"
        );
        // remove event is emitted
        app.update();
        let events = std::mem::take(&mut app.world.resource_mut::<StoredEvents>().0);
        let expected_events = vec![AssetEvent::Added { id }, AssetEvent::Removed { id }];
        assert_eq!(events, expected_events);

        let dep_handle = app.world.resource::<AssetServer>().load(dep_path);
        let a = CoolText {
            text: "a".to_string(),
            embedded: empty,
            // this dependency is behind a manual load gate, which should prevent 'a' from emitting a LoadedWithDependencies event
            dependencies: vec![dep_handle.clone()],
            sub_texts: Vec::new(),
        };
        let a_handle = app.world.resource::<AssetServer>().load_asset(a);
        app.update();
        // TODO: ideally it doesn't take two updates for the added event to emit
        app.update();

        let events = std::mem::take(&mut app.world.resource_mut::<StoredEvents>().0);
        let expected_events = vec![AssetEvent::Added { id: a_handle.id() }];
        assert_eq!(events, expected_events);

        gate_opener.open(dep_path);
        loop {
            app.update();
            let events = std::mem::take(&mut app.world.resource_mut::<StoredEvents>().0);
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
        let events = std::mem::take(&mut app.world.resource_mut::<StoredEvents>().0);
        let expected_events = vec![AssetEvent::Added {
            id: dep_handle.id(),
        }];
        assert_eq!(events, expected_events);
    }

    #[test]
    fn load_folder() {
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
        let asset_server = app.world.resource::<AssetServer>().clone();
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
}
