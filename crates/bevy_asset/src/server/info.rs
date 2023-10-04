use crate::{
    meta::{AssetHash, MetaTransform},
    Asset, AssetHandleProvider, AssetPath, DependencyLoadState, ErasedLoadedAsset, Handle,
    InternalAssetEvent, LoadState, RecursiveDependencyLoadState, StrongHandle, UntypedAssetId,
    UntypedHandle,
};
use bevy_ecs::world::World;
use bevy_log::warn;
use bevy_utils::{Entry, HashMap, HashSet};
use crossbeam_channel::Sender;
use std::{
    any::TypeId,
    sync::{Arc, Weak},
};
use thiserror::Error;

#[derive(Debug)]
pub(crate) struct AssetInfo {
    weak_handle: Weak<StrongHandle>,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) load_state: LoadState,
    pub(crate) dep_load_state: DependencyLoadState,
    pub(crate) rec_dep_load_state: RecursiveDependencyLoadState,
    loading_dependencies: HashSet<UntypedAssetId>,
    failed_dependencies: HashSet<UntypedAssetId>,
    loading_rec_dependencies: HashSet<UntypedAssetId>,
    failed_rec_dependencies: HashSet<UntypedAssetId>,
    dependants_waiting_on_load: HashSet<UntypedAssetId>,
    dependants_waiting_on_recursive_dep_load: HashSet<UntypedAssetId>,
    /// The asset paths required to load this asset. Hashes will only be set for processed assets.
    /// This is set using the value from [`LoadedAsset`].
    /// This will only be populated if [`AssetInfos::watching_for_changes`] is set to `true` to
    /// save memory.
    ///
    /// [`LoadedAsset`]: crate::loader::LoadedAsset
    loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    /// The number of handle drops to skip for this asset.
    /// See usage (and comments) in get_or_create_path_handle for context.
    handle_drops_to_skip: usize,
}

impl AssetInfo {
    fn new(weak_handle: Weak<StrongHandle>, path: Option<AssetPath<'static>>) -> Self {
        Self {
            weak_handle,
            path,
            load_state: LoadState::NotLoaded,
            dep_load_state: DependencyLoadState::NotLoaded,
            rec_dep_load_state: RecursiveDependencyLoadState::NotLoaded,
            loading_dependencies: HashSet::default(),
            failed_dependencies: HashSet::default(),
            loading_rec_dependencies: HashSet::default(),
            failed_rec_dependencies: HashSet::default(),
            loader_dependencies: HashMap::default(),
            dependants_waiting_on_load: HashSet::default(),
            dependants_waiting_on_recursive_dep_load: HashSet::default(),
            handle_drops_to_skip: 0,
        }
    }
}

#[derive(Default)]
pub(crate) struct AssetInfos {
    path_to_id: HashMap<AssetPath<'static>, UntypedAssetId>,
    infos: HashMap<UntypedAssetId, AssetInfo>,
    /// If set to `true`, this informs [`AssetInfos`] to track data relevant to watching for changes (such as `load_dependants`)
    /// This should only be set at startup.
    pub(crate) watching_for_changes: bool,
    /// Tracks assets that depend on the "key" asset path inside their asset loaders ("loader dependencies")
    /// This should only be set when watching for changes to avoid unnecessary work.
    pub(crate) loader_dependants: HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
    pub(crate) handle_providers: HashMap<TypeId, AssetHandleProvider>,
    pub(crate) dependency_loaded_event_sender: HashMap<TypeId, fn(&mut World, UntypedAssetId)>,
}

impl std::fmt::Debug for AssetInfos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetInfos")
            .field("path_to_id", &self.path_to_id)
            .field("infos", &self.infos)
            .finish()
    }
}

impl AssetInfos {
    pub(crate) fn create_loading_handle<A: Asset>(&mut self) -> Handle<A> {
        unwrap_with_context(
            Self::create_handle_internal(
                &mut self.infos,
                &self.handle_providers,
                TypeId::of::<A>(),
                None,
                None,
                true,
            ),
            std::any::type_name::<A>(),
        )
        .typed_debug_checked()
    }

    pub(crate) fn create_loading_handle_untyped(
        &mut self,
        type_id: TypeId,
        type_name: &'static str,
    ) -> UntypedHandle {
        unwrap_with_context(
            Self::create_handle_internal(
                &mut self.infos,
                &self.handle_providers,
                type_id,
                None,
                None,
                true,
            ),
            type_name,
        )
    }

    fn create_handle_internal(
        infos: &mut HashMap<UntypedAssetId, AssetInfo>,
        handle_providers: &HashMap<TypeId, AssetHandleProvider>,
        type_id: TypeId,
        path: Option<AssetPath<'static>>,
        meta_transform: Option<MetaTransform>,
        loading: bool,
    ) -> Result<UntypedHandle, MissingHandleProviderError> {
        let provider = handle_providers
            .get(&type_id)
            .ok_or(MissingHandleProviderError(type_id))?;

        let handle = provider.reserve_handle_internal(true, path.clone(), meta_transform);
        let mut info = AssetInfo::new(Arc::downgrade(&handle), path);
        if loading {
            info.load_state = LoadState::Loading;
            info.dep_load_state = DependencyLoadState::Loading;
            info.rec_dep_load_state = RecursiveDependencyLoadState::Loading;
        }
        infos.insert(handle.id, info);
        Ok(UntypedHandle::Strong(handle))
    }

    pub(crate) fn get_or_create_path_handle<A: Asset>(
        &mut self,
        path: AssetPath<'static>,
        loading_mode: HandleLoadingMode,
        meta_transform: Option<MetaTransform>,
    ) -> (Handle<A>, bool) {
        let result = self.get_or_create_path_handle_internal(
            path,
            TypeId::of::<A>(),
            loading_mode,
            meta_transform,
        );
        let (handle, should_load) = unwrap_with_context(result, std::any::type_name::<A>());
        (handle.typed_unchecked(), should_load)
    }

    pub(crate) fn get_or_create_path_handle_untyped(
        &mut self,
        path: AssetPath<'static>,
        type_id: TypeId,
        type_name: &'static str,
        loading_mode: HandleLoadingMode,
        meta_transform: Option<MetaTransform>,
    ) -> (UntypedHandle, bool) {
        let result =
            self.get_or_create_path_handle_internal(path, type_id, loading_mode, meta_transform);
        unwrap_with_context(result, type_name)
    }

    /// Retrieves asset tracking data, or creates it if it doesn't exist.
    /// Returns true if an asset load should be kicked off
    pub fn get_or_create_path_handle_internal(
        &mut self,
        path: AssetPath<'static>,
        type_id: TypeId,
        loading_mode: HandleLoadingMode,
        meta_transform: Option<MetaTransform>,
    ) -> Result<(UntypedHandle, bool), MissingHandleProviderError> {
        match self.path_to_id.entry(path.clone()) {
            Entry::Occupied(entry) => {
                let id = *entry.get();
                // if there is a path_to_id entry, info always exists
                let info = self.infos.get_mut(&id).unwrap();
                let mut should_load = false;
                if loading_mode == HandleLoadingMode::Force
                    || (loading_mode == HandleLoadingMode::Request
                        && info.load_state == LoadState::NotLoaded)
                {
                    info.load_state = LoadState::Loading;
                    info.dep_load_state = DependencyLoadState::Loading;
                    info.rec_dep_load_state = RecursiveDependencyLoadState::Loading;
                    should_load = true;
                }

                if let Some(strong_handle) = info.weak_handle.upgrade() {
                    // If we can upgrade the handle, there is at least one live handle right now,
                    // The asset load has already kicked off (and maybe completed), so we can just
                    // return a strong handle
                    Ok((UntypedHandle::Strong(strong_handle), should_load))
                } else {
                    // Asset meta exists, but all live handles were dropped. This means the `track_assets` system
                    // hasn't been run yet to remove the current asset
                    // (note that this is guaranteed to be transactional with the `track_assets` system because
                    // because it locks the AssetInfos collection)

                    // We must create a new strong handle for the existing id and ensure that the drop of the old
                    // strong handle doesn't remove the asset from the Assets collection
                    info.handle_drops_to_skip += 1;
                    let provider = self
                        .handle_providers
                        .get(&type_id)
                        .ok_or(MissingHandleProviderError(type_id))?;
                    let handle =
                        provider.get_handle(id.internal(), true, Some(path), meta_transform);
                    info.weak_handle = Arc::downgrade(&handle);
                    Ok((UntypedHandle::Strong(handle), should_load))
                }
            }
            // The entry does not exist, so this is a "fresh" asset load. We must create a new handle
            Entry::Vacant(entry) => {
                let should_load = match loading_mode {
                    HandleLoadingMode::NotLoading => false,
                    HandleLoadingMode::Request | HandleLoadingMode::Force => true,
                };
                let handle = Self::create_handle_internal(
                    &mut self.infos,
                    &self.handle_providers,
                    type_id,
                    Some(path),
                    meta_transform,
                    should_load,
                )?;
                entry.insert(handle.id());
                Ok((handle, should_load))
            }
        }
    }

    pub(crate) fn get(&self, id: UntypedAssetId) -> Option<&AssetInfo> {
        self.infos.get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: UntypedAssetId) -> Option<&mut AssetInfo> {
        self.infos.get_mut(&id)
    }

    pub(crate) fn get_path_handle(&self, path: AssetPath) -> Option<UntypedHandle> {
        let id = *self.path_to_id.get(&path)?;
        self.get_id_handle(id)
    }

    pub(crate) fn get_id_handle(&self, id: UntypedAssetId) -> Option<UntypedHandle> {
        let info = self.infos.get(&id)?;
        let strong_handle = info.weak_handle.upgrade()?;
        Some(UntypedHandle::Strong(strong_handle))
    }

    /// Returns `true` if this path has
    pub(crate) fn is_path_alive<'a>(&self, path: impl Into<AssetPath<'a>>) -> bool {
        let path = path.into();
        if let Some(id) = self.path_to_id.get(&path) {
            if let Some(info) = self.infos.get(id) {
                return info.weak_handle.strong_count() > 0;
            }
        }
        false
    }

    // Returns `true` if the asset should be removed from the collection
    pub(crate) fn process_handle_drop(&mut self, id: UntypedAssetId) -> bool {
        Self::process_handle_drop_internal(
            &mut self.infos,
            &mut self.path_to_id,
            &mut self.loader_dependants,
            self.watching_for_changes,
            id,
        )
    }

    /// Updates [`AssetInfo`] / load state for an asset that has finished loading (and relevant dependencies / dependants).
    pub(crate) fn process_asset_load(
        &mut self,
        loaded_asset_id: UntypedAssetId,
        loaded_asset: ErasedLoadedAsset,
        world: &mut World,
        sender: &Sender<InternalAssetEvent>,
    ) {
        loaded_asset.value.insert(loaded_asset_id, world);
        let mut loading_deps = loaded_asset.dependencies;
        let mut failed_deps = HashSet::new();
        let mut loading_rec_deps = loading_deps.clone();
        let mut failed_rec_deps = HashSet::new();
        loading_deps.retain(|dep_id| {
            if let Some(dep_info) = self.get_mut(*dep_id) {
                match dep_info.rec_dep_load_state {
                    RecursiveDependencyLoadState::Loading
                    | RecursiveDependencyLoadState::NotLoaded => {
                        // If dependency is loading, wait for it.
                        dep_info
                            .dependants_waiting_on_recursive_dep_load
                            .insert(loaded_asset_id);
                    }
                    RecursiveDependencyLoadState::Loaded => {
                        // If dependency is loaded, reduce our count by one
                        loading_rec_deps.remove(dep_id);
                    }
                    RecursiveDependencyLoadState::Failed => {
                        failed_rec_deps.insert(*dep_id);
                        loading_rec_deps.remove(dep_id);
                    }
                }
                match dep_info.load_state {
                    LoadState::NotLoaded | LoadState::Loading => {
                        // If dependency is loading, wait for it.
                        dep_info.dependants_waiting_on_load.insert(loaded_asset_id);
                        true
                    }
                    LoadState::Loaded => {
                        // If dependency is loaded, reduce our count by one
                        false
                    }
                    LoadState::Failed => {
                        failed_deps.insert(*dep_id);
                        false
                    }
                }
            } else {
                // the dependency id does not exist, which implies it was manually removed or never existed in the first place
                warn!(
                    "Dependency {:?} from asset {:?} is unknown. This asset's dependency load status will not switch to 'Loaded' until the unknown dependency is loaded.",
                    dep_id, loaded_asset_id
                );
                true
            }
        });

        let dep_load_state = match (loading_deps.len(), failed_deps.len()) {
            (0, 0) => DependencyLoadState::Loaded,
            (_loading, 0) => DependencyLoadState::Loading,
            (_loading, _failed) => DependencyLoadState::Failed,
        };

        let rec_dep_load_state = match (loading_rec_deps.len(), failed_rec_deps.len()) {
            (0, 0) => {
                sender
                    .send(InternalAssetEvent::LoadedWithDependencies {
                        id: loaded_asset_id,
                    })
                    .unwrap();
                RecursiveDependencyLoadState::Loaded
            }
            (_loading, 0) => RecursiveDependencyLoadState::Loading,
            (_loading, _failed) => RecursiveDependencyLoadState::Failed,
        };

        let (dependants_waiting_on_load, dependants_waiting_on_rec_load) = {
            let watching_for_changes = self.watching_for_changes;
            // if watching for changes, track reverse loader dependencies for hot reloading
            if watching_for_changes {
                let info = self
                    .infos
                    .get(&loaded_asset_id)
                    .expect("Asset info should always exist at this point");
                if let Some(asset_path) = &info.path {
                    for loader_dependency in loaded_asset.loader_dependencies.keys() {
                        let dependants = self
                            .loader_dependants
                            .entry(loader_dependency.clone())
                            .or_default();
                        dependants.insert(asset_path.clone());
                    }
                }
            }
            let info = self
                .get_mut(loaded_asset_id)
                .expect("Asset info should always exist at this point");
            info.loading_dependencies = loading_deps;
            info.failed_dependencies = failed_deps;
            info.loading_rec_dependencies = loading_rec_deps;
            info.failed_rec_dependencies = failed_rec_deps;
            info.load_state = LoadState::Loaded;
            info.dep_load_state = dep_load_state;
            info.rec_dep_load_state = rec_dep_load_state;
            if watching_for_changes {
                info.loader_dependencies = loaded_asset.loader_dependencies;
            }

            let dependants_waiting_on_rec_load = if matches!(
                rec_dep_load_state,
                RecursiveDependencyLoadState::Loaded | RecursiveDependencyLoadState::Failed
            ) {
                Some(std::mem::take(
                    &mut info.dependants_waiting_on_recursive_dep_load,
                ))
            } else {
                None
            };

            (
                std::mem::take(&mut info.dependants_waiting_on_load),
                dependants_waiting_on_rec_load,
            )
        };

        for id in dependants_waiting_on_load {
            if let Some(info) = self.get_mut(id) {
                info.loading_dependencies.remove(&loaded_asset_id);
                if info.loading_dependencies.is_empty() {
                    // send dependencies loaded event
                    info.dep_load_state = DependencyLoadState::Loaded;
                }
            }
        }

        if let Some(dependants_waiting_on_rec_load) = dependants_waiting_on_rec_load {
            match rec_dep_load_state {
                RecursiveDependencyLoadState::Loaded => {
                    for dep_id in dependants_waiting_on_rec_load {
                        Self::propagate_loaded_state(self, loaded_asset_id, dep_id, sender);
                    }
                }
                RecursiveDependencyLoadState::Failed => {
                    for dep_id in dependants_waiting_on_rec_load {
                        Self::propagate_failed_state(self, loaded_asset_id, dep_id);
                    }
                }
                RecursiveDependencyLoadState::Loading | RecursiveDependencyLoadState::NotLoaded => {
                    // dependants_waiting_on_rec_load should be None in this case
                    unreachable!("`Loading` and `NotLoaded` state should never be propagated.")
                }
            }
        }
    }

    /// Recursively propagates loaded state up the dependency tree.
    fn propagate_loaded_state(
        infos: &mut AssetInfos,
        loaded_id: UntypedAssetId,
        waiting_id: UntypedAssetId,
        sender: &Sender<InternalAssetEvent>,
    ) {
        let dependants_waiting_on_rec_load = if let Some(info) = infos.get_mut(waiting_id) {
            info.loading_rec_dependencies.remove(&loaded_id);
            if info.loading_rec_dependencies.is_empty() && info.failed_rec_dependencies.is_empty() {
                info.rec_dep_load_state = RecursiveDependencyLoadState::Loaded;
                if info.load_state == LoadState::Loaded {
                    sender
                        .send(InternalAssetEvent::LoadedWithDependencies { id: waiting_id })
                        .unwrap();
                }
                Some(std::mem::take(
                    &mut info.dependants_waiting_on_recursive_dep_load,
                ))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(dependants_waiting_on_rec_load) = dependants_waiting_on_rec_load {
            for dep_id in dependants_waiting_on_rec_load {
                Self::propagate_loaded_state(infos, waiting_id, dep_id, sender);
            }
        }
    }

    /// Recursively propagates failed state up the dependency tree
    fn propagate_failed_state(
        infos: &mut AssetInfos,
        failed_id: UntypedAssetId,
        waiting_id: UntypedAssetId,
    ) {
        let dependants_waiting_on_rec_load = if let Some(info) = infos.get_mut(waiting_id) {
            info.loading_rec_dependencies.remove(&failed_id);
            info.failed_rec_dependencies.insert(failed_id);
            info.rec_dep_load_state = RecursiveDependencyLoadState::Failed;
            Some(std::mem::take(
                &mut info.dependants_waiting_on_recursive_dep_load,
            ))
        } else {
            None
        };

        if let Some(dependants_waiting_on_rec_load) = dependants_waiting_on_rec_load {
            for dep_id in dependants_waiting_on_rec_load {
                Self::propagate_failed_state(infos, waiting_id, dep_id);
            }
        }
    }

    pub(crate) fn process_asset_fail(&mut self, failed_id: UntypedAssetId) {
        let (dependants_waiting_on_load, dependants_waiting_on_rec_load) = {
            let info = self
                .get_mut(failed_id)
                .expect("Asset info should always exist at this point");
            info.load_state = LoadState::Failed;
            info.dep_load_state = DependencyLoadState::Failed;
            info.rec_dep_load_state = RecursiveDependencyLoadState::Failed;
            (
                std::mem::take(&mut info.dependants_waiting_on_load),
                std::mem::take(&mut info.dependants_waiting_on_recursive_dep_load),
            )
        };

        for waiting_id in dependants_waiting_on_load {
            if let Some(info) = self.get_mut(waiting_id) {
                info.loading_dependencies.remove(&failed_id);
                info.failed_dependencies.insert(failed_id);
                info.dep_load_state = DependencyLoadState::Failed;
            }
        }

        for waiting_id in dependants_waiting_on_rec_load {
            Self::propagate_failed_state(self, failed_id, waiting_id);
        }
    }

    fn process_handle_drop_internal(
        infos: &mut HashMap<UntypedAssetId, AssetInfo>,
        path_to_id: &mut HashMap<AssetPath<'static>, UntypedAssetId>,
        loader_dependants: &mut HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
        watching_for_changes: bool,
        id: UntypedAssetId,
    ) -> bool {
        match infos.entry(id) {
            Entry::Occupied(mut entry) => {
                if entry.get_mut().handle_drops_to_skip > 0 {
                    entry.get_mut().handle_drops_to_skip -= 1;
                    false
                } else {
                    let info = entry.remove();
                    if let Some(path) = info.path {
                        if watching_for_changes {
                            for loader_dependency in info.loader_dependencies.keys() {
                                if let Some(dependants) =
                                    loader_dependants.get_mut(loader_dependency)
                                {
                                    dependants.remove(&path);
                                }
                            }
                        }
                        path_to_id.remove(&path);
                    }
                    true
                }
            }
            // Either the asset was already dropped, it doesn't exist, or it isn't managed by the asset server
            // None of these cases should result in a removal from the Assets collection
            Entry::Vacant(_) => false,
        }
    }

    /// Consumes all current handle drop events. This will update information in [`AssetInfos`], but it
    /// will not affect [`Assets`] storages. For normal use cases, prefer `Assets::track_assets()`
    /// This should only be called if `Assets` storage isn't being used (such as in [`AssetProcessor`](crate::processor::AssetProcessor))
    ///
    /// [`Assets`]: crate::Assets
    pub(crate) fn consume_handle_drop_events(&mut self) {
        for provider in self.handle_providers.values() {
            while let Ok(drop_event) = provider.drop_receiver.try_recv() {
                let id = drop_event.id;
                if drop_event.asset_server_managed {
                    Self::process_handle_drop_internal(
                        &mut self.infos,
                        &mut self.path_to_id,
                        &mut self.loader_dependants,
                        self.watching_for_changes,
                        id.untyped(provider.type_id),
                    );
                }
            }
        }
    }
}

/// Determines how a handle should be initialized
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum HandleLoadingMode {
    /// The handle is for an asset that isn't loading/loaded yet.
    NotLoading,
    /// The handle is for an asset that is being _requested_ to load (if it isn't already loading)
    Request,
    /// The handle is for an asset that is being forced to load (even if it has already loaded)
    Force,
}

#[derive(Error, Debug)]
#[error("Cannot allocate a handle because no handle provider exists for asset type {0:?}")]
pub struct MissingHandleProviderError(TypeId);

fn unwrap_with_context<T>(
    result: Result<T, MissingHandleProviderError>,
    type_name: &'static str,
) -> T {
    match result {
        Ok(value) => value,
        Err(_) => {
            panic!("Cannot allocate an Asset Handle of type '{type_name}' because the asset type has not been initialized. \
                    Make sure you have called app.init_asset::<{type_name}>()")
        }
    }
}
