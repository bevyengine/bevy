use crate::{
    meta::{AssetHash, MetaTransform},
    Asset, AssetHandleProvider, AssetIndex, AssetLoadError, AssetPath, DependencyLoadState,
    ErasedAssetIndex, ErasedLoadedAsset, Handle, HandleEvent, InternalAssetEvent, LoadState,
    RecursiveDependencyLoadState, StrongHandle, UntypedHandle,
};
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    sync::{Arc, Weak},
    vec::Vec,
};
use bevy_ecs::world::World;
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_tasks::Task;
use bevy_utils::TypeIdMap;
use core::{any::TypeId, task::Waker};
use crossbeam_channel::Sender;
use either::Either;
use thiserror::Error;
use tracing::warn;

#[derive(Debug)]
pub(crate) struct AssetInfo {
    weak_handle: Weak<StrongHandle>,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) load_state: LoadState,
    pub(crate) dep_load_state: DependencyLoadState,
    pub(crate) rec_dep_load_state: RecursiveDependencyLoadState,
    loading_dependencies: HashSet<ErasedAssetIndex>,
    failed_dependencies: HashSet<ErasedAssetIndex>,
    loading_rec_dependencies: HashSet<ErasedAssetIndex>,
    failed_rec_dependencies: HashSet<ErasedAssetIndex>,
    dependents_waiting_on_load: HashSet<ErasedAssetIndex>,
    dependents_waiting_on_recursive_dep_load: HashSet<ErasedAssetIndex>,
    /// The asset paths required to load this asset. Hashes will only be set for processed assets.
    /// This is set using the value from [`LoadedAsset`].
    /// This will only be populated if [`AssetInfos::watching_for_changes`] is set to `true` to
    /// save memory.
    ///
    /// [`LoadedAsset`]: crate::loader::LoadedAsset
    loader_dependencies: HashMap<AssetPath<'static>, AssetHash>,
    /// List of tasks waiting for this asset to complete loading
    pub(crate) waiting_tasks: Vec<Waker>,
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
            dependents_waiting_on_load: HashSet::default(),
            dependents_waiting_on_recursive_dep_load: HashSet::default(),
            waiting_tasks: Vec::new(),
        }
    }
}

/// Tracks statistics of the asset server.
#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct AssetServerStats {
    /// The number of load tasks that have been started.
    pub(crate) started_load_tasks: usize,
}

#[derive(Default)]
pub(crate) struct AssetInfos {
    path_to_index: HashMap<AssetPath<'static>, TypeIdMap<AssetIndex>>,
    infos: HashMap<ErasedAssetIndex, AssetInfo>,
    /// If set to `true`, this informs [`AssetInfos`] to track data relevant to watching for changes (such as `load_dependents`)
    /// This should only be set at startup.
    pub(crate) watching_for_changes: bool,
    /// Tracks assets that depend on the "key" asset path inside their asset loaders ("loader dependencies")
    /// This should only be set when watching for changes to avoid unnecessary work.
    pub(crate) loader_dependents: HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
    /// Tracks living labeled assets for a given source asset.
    /// This should only be set when watching for changes to avoid unnecessary work.
    pub(crate) living_labeled_assets: HashMap<AssetPath<'static>, HashSet<Box<str>>>,
    pub(crate) handle_providers: TypeIdMap<AssetHandleProvider>,
    pub(crate) dependency_loaded_event_sender: TypeIdMap<fn(&mut World, AssetIndex)>,
    pub(crate) dependency_failed_event_sender:
        TypeIdMap<fn(&mut World, AssetIndex, AssetPath<'static>, AssetLoadError)>,
    pub(crate) pending_tasks: HashMap<ErasedAssetIndex, Task<()>>,
    /// The stats that have collected during usage of the asset server.
    pub(crate) stats: AssetServerStats,
}

impl core::fmt::Debug for AssetInfos {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AssetInfos")
            .field("path_to_index", &self.path_to_index)
            .field("infos", &self.infos)
            .finish()
    }
}

impl AssetInfos {
    pub(crate) fn create_loading_handle_untyped(
        &mut self,
        type_id: TypeId,
        type_name: &'static str,
    ) -> UntypedHandle {
        unwrap_with_context(
            Self::create_handle_internal(
                &mut self.infos,
                &self.handle_providers,
                &mut self.living_labeled_assets,
                self.watching_for_changes,
                type_id,
                None,
                None,
                true,
            ),
            Either::Left(type_name),
        )
        .unwrap()
    }

    fn create_handle_internal(
        infos: &mut HashMap<ErasedAssetIndex, AssetInfo>,
        handle_providers: &TypeIdMap<AssetHandleProvider>,
        living_labeled_assets: &mut HashMap<AssetPath<'static>, HashSet<Box<str>>>,
        watching_for_changes: bool,
        type_id: TypeId,
        path: Option<AssetPath<'static>>,
        meta_transform: Option<MetaTransform>,
        loading: bool,
    ) -> Result<UntypedHandle, GetOrCreateHandleInternalError> {
        let provider = handle_providers
            .get(&type_id)
            .ok_or(MissingHandleProviderError(type_id))?;

        if watching_for_changes && let Some(path) = &path {
            let mut without_label = path.to_owned();
            if let Some(label) = without_label.take_label() {
                let labels = living_labeled_assets.entry(without_label).or_default();
                labels.insert(label.as_ref().into());
            }
        }

        let handle = provider.reserve_handle_internal(path.clone(), meta_transform);
        let mut info = AssetInfo::new(Arc::downgrade(&handle), path);
        if loading {
            info.load_state = LoadState::Loading;
            info.dep_load_state = DependencyLoadState::Loading;
            info.rec_dep_load_state = RecursiveDependencyLoadState::Loading;
        }
        infos.insert(ErasedAssetIndex::new(handle.index, handle.type_id), info);

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
            Some(TypeId::of::<A>()),
            loading_mode,
            meta_transform,
        );
        // it is ok to unwrap because TypeId was specified above
        let (handle, should_load) =
            unwrap_with_context(result, Either::Left(core::any::type_name::<A>())).unwrap();
        (handle.typed_unchecked(), should_load)
    }

    pub(crate) fn get_or_create_path_handle_erased(
        &mut self,
        path: AssetPath<'static>,
        type_id: TypeId,
        type_name: Option<&str>,
        loading_mode: HandleLoadingMode,
        meta_transform: Option<MetaTransform>,
    ) -> (UntypedHandle, bool) {
        let result = self.get_or_create_path_handle_internal(
            path,
            Some(type_id),
            loading_mode,
            meta_transform,
        );
        let type_info = match type_name {
            Some(type_name) => Either::Left(type_name),
            None => Either::Right(type_id),
        };
        unwrap_with_context(result, type_info)
            .expect("type should be correct since the `TypeId` is specified above")
    }

    /// Retrieves asset tracking data, or creates it if it doesn't exist.
    /// Returns true if an asset load should be kicked off
    pub(crate) fn get_or_create_path_handle_internal(
        &mut self,
        path: AssetPath<'static>,
        type_id: Option<TypeId>,
        loading_mode: HandleLoadingMode,
        meta_transform: Option<MetaTransform>,
    ) -> Result<(UntypedHandle, bool), GetOrCreateHandleInternalError> {
        let handles = self.path_to_index.entry(path.clone()).or_default();

        let type_id = type_id
            .or_else(|| {
                // If a TypeId is not provided, we may be able to infer it if only a single entry exists
                if handles.len() == 1 {
                    Some(*handles.keys().next().unwrap())
                } else {
                    None
                }
            })
            .ok_or(GetOrCreateHandleInternalError::HandleMissingButTypeIdNotSpecified)?;

        match handles.entry(type_id) {
            Entry::Occupied(entry) => {
                let index = *entry.get();
                // if there is a path_to_id entry, info always exists
                let info = self
                    .infos
                    .get_mut(&ErasedAssetIndex::new(index, type_id))
                    .unwrap();
                let mut should_load = false;
                if loading_mode == HandleLoadingMode::Force
                    || (loading_mode == HandleLoadingMode::Request
                        && matches!(info.load_state, LoadState::NotLoaded | LoadState::Failed(_)))
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
                    // (note that this is guaranteed to be transactional with the `track_assets` system
                    // because it locks the AssetInfos collection)

                    // We must create a new strong handle for the existing id.
                    let provider = self
                        .handle_providers
                        .get(&type_id)
                        .ok_or(MissingHandleProviderError(type_id))?;
                    let handle = provider.get_handle(index, Some(path), meta_transform);
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
                    &mut self.living_labeled_assets,
                    self.watching_for_changes,
                    type_id,
                    Some(path),
                    meta_transform,
                    should_load,
                )?;
                let index = match &handle {
                    UntypedHandle::Strong(handle) => handle.index,
                    // `create_handle_internal` always returns Strong variant.
                    UntypedHandle::Uuid { .. } => unreachable!(),
                };
                entry.insert(index);
                Ok((handle, should_load))
            }
        }
    }

    pub(crate) fn get(&self, index: ErasedAssetIndex) -> Option<&AssetInfo> {
        self.infos.get(&index)
    }

    pub(crate) fn contains_key(&self, index: ErasedAssetIndex) -> bool {
        self.infos.contains_key(&index)
    }

    pub(crate) fn get_mut(&mut self, index: ErasedAssetIndex) -> Option<&mut AssetInfo> {
        self.infos.get_mut(&index)
    }

    pub(crate) fn get_path_and_type_id_handle(
        &mut self,
        path: &AssetPath<'_>,
        type_id: TypeId,
    ) -> Option<UntypedHandle> {
        let index = *self.path_to_index.get(path)?.get(&type_id)?;
        self.get_index_handle(ErasedAssetIndex::new(index, type_id))
    }

    pub(crate) fn get_path_indices<'a>(
        &'a self,
        path: &'a AssetPath<'_>,
    ) -> impl Iterator<Item = ErasedAssetIndex> + 'a {
        self.path_to_index
            .get(path)
            .into_iter()
            .flat_map(|type_id_to_index| type_id_to_index.iter())
            .map(|(type_id, index)| ErasedAssetIndex::new(*index, *type_id))
    }

    pub(crate) fn get_path_handles<'a>(
        &'a mut self,
        path: &'a AssetPath<'_>,
    ) -> impl Iterator<Item = UntypedHandle> + 'a {
        self.path_to_index
            .get(path)
            .into_iter()
            .flat_map(|type_id_to_index| type_id_to_index.iter())
            .map(|(type_id, index)| ErasedAssetIndex::new(*index, *type_id))
            .filter_map(|index| {
                Self::get_index_handle_internal(&mut self.infos, &self.handle_providers, index)
            })
    }

    pub(crate) fn get_index_handle(&mut self, index: ErasedAssetIndex) -> Option<UntypedHandle> {
        Self::get_index_handle_internal(&mut self.infos, &self.handle_providers, index)
    }

    /// Same as [`Self::get_index_handle`], but allows passing in only the required fields, to allow
    /// holding borrows to other fields.
    fn get_index_handle_internal(
        infos: &mut HashMap<ErasedAssetIndex, AssetInfo>,
        handle_providers: &TypeIdMap<AssetHandleProvider>,
        index: ErasedAssetIndex,
    ) -> Option<UntypedHandle> {
        let info = infos.get_mut(&index)?;
        let strong_handle = match info.weak_handle.upgrade() {
            Some(handle) => handle,
            None => {
                // We still have this index's entry, so it hasn't been dropped yet, so we can
                // allocate a new handle and return it.
                let provider = handle_providers.get(&index.type_id).expect(
                    "asset was loaded before, so we must have a corresponding handle_provider",
                );
                let handle = provider.get_handle(index.index, info.path.clone(), None);
                info.weak_handle = Arc::downgrade(&handle);
                handle
            }
        };
        Some(UntypedHandle::Strong(strong_handle))
    }

    /// Returns `true` if the asset at this path should be reloaded
    pub(crate) fn should_reload(&self, path: &AssetPath) -> bool {
        // If any path indices still exist, that means they haven't been dropped yet, so we should
        // try to reload this path. Technically, there's a race condition where all handles may have
        // been dropped, but `Assets::track_assets` hasn't run yet, so we'd be reloading an asset
        // that will be dropped soon. This will result in a load which is unfortunate, but not too
        // big a deal, and should be very rare.
        if self.get_path_indices(&path.into()).next().is_some() {
            return true;
        }

        if let Some(living) = self.living_labeled_assets.get(path) {
            !living.is_empty()
        } else {
            false
        }
    }

    /// Handles when an asset entry has been dropped from [`crate::Assets`].
    pub(crate) fn process_asset_entry_drop(&mut self, index: ErasedAssetIndex) {
        Self::process_asset_entry_drop_internal(
            &mut self.infos,
            &mut self.path_to_index,
            &mut self.loader_dependents,
            &mut self.living_labeled_assets,
            &mut self.pending_tasks,
            self.watching_for_changes,
            index,
        );
    }

    /// Updates [`AssetInfo`] / load state for an asset that has finished loading (and relevant dependencies / dependents).
    pub(crate) fn process_asset_load(
        &mut self,
        loaded_asset_index: ErasedAssetIndex,
        loaded_asset: ErasedLoadedAsset,
        world: &mut World,
        sender: &Sender<InternalAssetEvent>,
    ) {
        // Process all the labeled assets first so that they don't get skipped due to the "parent"
        // not having its handle alive.
        for (_, asset) in loaded_asset.labeled_assets {
            let UntypedHandle::Strong(handle) = &asset.handle else {
                unreachable!("Labeled assets are always strong handles");
            };
            self.process_asset_load(
                ErasedAssetIndex {
                    index: handle.index,
                    type_id: handle.type_id,
                },
                asset.asset,
                world,
                sender,
            );
        }

        // Check whether the handle has been dropped since the asset was loaded.
        if !self.infos.contains_key(&loaded_asset_index) {
            return;
        }

        loaded_asset.value.insert(loaded_asset_index.index, world);
        let mut loading_deps = loaded_asset.dependencies;
        let mut failed_deps = <HashSet<_>>::default();
        let mut dep_error = None;
        let mut loading_rec_deps = loading_deps.clone();
        let mut failed_rec_deps = <HashSet<_>>::default();
        let mut rec_dep_error = None;
        loading_deps.retain(|dep_id| {
            if let Some(dep_info) = self.get_mut(*dep_id) {
                match dep_info.rec_dep_load_state {
                    RecursiveDependencyLoadState::Loading
                    | RecursiveDependencyLoadState::NotLoaded => {
                        // If dependency is loading, wait for it.
                        dep_info
                            .dependents_waiting_on_recursive_dep_load
                            .insert(loaded_asset_index);
                    }
                    RecursiveDependencyLoadState::Loaded => {
                        // If dependency is loaded, reduce our count by one
                        loading_rec_deps.remove(dep_id);
                    }
                    RecursiveDependencyLoadState::Failed(ref error) => {
                        if rec_dep_error.is_none() {
                            rec_dep_error = Some(error.clone());
                        }
                        failed_rec_deps.insert(*dep_id);
                        loading_rec_deps.remove(dep_id);
                    }
                }
                match dep_info.load_state {
                    LoadState::NotLoaded | LoadState::Loading => {
                        // If dependency is loading, wait for it.
                        dep_info.dependents_waiting_on_load.insert(loaded_asset_index);
                        true
                    }
                    LoadState::Loaded => {
                        // If dependency is loaded, reduce our count by one
                        false
                    }
                    LoadState::Failed(ref error) => {
                        if dep_error.is_none() {
                            dep_error = Some(error.clone());
                        }
                        failed_deps.insert(*dep_id);
                        false
                    }
                }
            } else {
                // the dependency id does not exist, which implies it was manually removed or never existed in the first place
                warn!(
                    "Dependency {} from asset {} is unknown. This asset's dependency load status will not switch to 'Loaded' until the unknown dependency is loaded.",
                    dep_id, loaded_asset_index
                );
                true
            }
        });

        let dep_load_state = match (loading_deps.len(), failed_deps.len()) {
            (0, 0) => DependencyLoadState::Loaded,
            (_loading, 0) => DependencyLoadState::Loading,
            (_loading, _failed) => DependencyLoadState::Failed(dep_error.unwrap()),
        };

        let rec_dep_load_state = match (loading_rec_deps.len(), failed_rec_deps.len()) {
            (0, 0) => {
                sender
                    .send(InternalAssetEvent::LoadedWithDependencies {
                        index: loaded_asset_index,
                    })
                    .unwrap();
                RecursiveDependencyLoadState::Loaded
            }
            (_loading, 0) => RecursiveDependencyLoadState::Loading,
            (_loading, _failed) => RecursiveDependencyLoadState::Failed(rec_dep_error.unwrap()),
        };

        let (dependents_waiting_on_load, dependents_waiting_on_rec_load) = {
            let watching_for_changes = self.watching_for_changes;
            // if watching for changes, track reverse loader dependencies for hot reloading
            if watching_for_changes {
                let info = self
                    .infos
                    .get(&loaded_asset_index)
                    .expect("Asset info should always exist at this point");
                if let Some(asset_path) = &info.path {
                    for loader_dependency in loaded_asset.loader_dependencies.keys() {
                        let dependents = self
                            .loader_dependents
                            .entry(loader_dependency.clone())
                            .or_default();
                        dependents.insert(asset_path.clone());
                    }
                }
            }
            let info = self
                .get_mut(loaded_asset_index)
                .expect("Asset info should always exist at this point");
            info.loading_dependencies = loading_deps;
            info.failed_dependencies = failed_deps;
            info.loading_rec_dependencies = loading_rec_deps;
            info.failed_rec_dependencies = failed_rec_deps;
            info.load_state = LoadState::Loaded;
            info.dep_load_state = dep_load_state;
            info.rec_dep_load_state = rec_dep_load_state.clone();
            if watching_for_changes {
                info.loader_dependencies = loaded_asset.loader_dependencies;
            }

            let dependents_waiting_on_rec_load =
                if rec_dep_load_state.is_loaded() || rec_dep_load_state.is_failed() {
                    Some(core::mem::take(
                        &mut info.dependents_waiting_on_recursive_dep_load,
                    ))
                } else {
                    None
                };

            (
                core::mem::take(&mut info.dependents_waiting_on_load),
                dependents_waiting_on_rec_load,
            )
        };

        for id in dependents_waiting_on_load {
            if let Some(info) = self.get_mut(id) {
                info.loading_dependencies.remove(&loaded_asset_index);
                if info.loading_dependencies.is_empty() && !info.dep_load_state.is_failed() {
                    // send dependencies loaded event
                    info.dep_load_state = DependencyLoadState::Loaded;
                }
            }
        }

        if let Some(dependents_waiting_on_rec_load) = dependents_waiting_on_rec_load {
            match rec_dep_load_state {
                RecursiveDependencyLoadState::Loaded => {
                    for dep_id in dependents_waiting_on_rec_load {
                        Self::propagate_loaded_state(self, loaded_asset_index, dep_id, sender);
                    }
                }
                RecursiveDependencyLoadState::Failed(ref error) => {
                    for dep_id in dependents_waiting_on_rec_load {
                        Self::propagate_failed_state(self, loaded_asset_index, dep_id, error);
                    }
                }
                RecursiveDependencyLoadState::Loading | RecursiveDependencyLoadState::NotLoaded => {
                    // dependents_waiting_on_rec_load should be None in this case
                    unreachable!("`Loading` and `NotLoaded` state should never be propagated.")
                }
            }
        }
    }

    /// Recursively propagates loaded state up the dependency tree.
    fn propagate_loaded_state(
        infos: &mut AssetInfos,
        loaded_id: ErasedAssetIndex,
        waiting_id: ErasedAssetIndex,
        sender: &Sender<InternalAssetEvent>,
    ) {
        let dependents_waiting_on_rec_load = if let Some(info) = infos.get_mut(waiting_id) {
            info.loading_rec_dependencies.remove(&loaded_id);
            if info.loading_rec_dependencies.is_empty() && info.failed_rec_dependencies.is_empty() {
                info.rec_dep_load_state = RecursiveDependencyLoadState::Loaded;
                if info.load_state.is_loaded() {
                    sender
                        .send(InternalAssetEvent::LoadedWithDependencies { index: waiting_id })
                        .unwrap();
                }
                Some(core::mem::take(
                    &mut info.dependents_waiting_on_recursive_dep_load,
                ))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(dependents_waiting_on_rec_load) = dependents_waiting_on_rec_load {
            for dep_id in dependents_waiting_on_rec_load {
                Self::propagate_loaded_state(infos, waiting_id, dep_id, sender);
            }
        }
    }

    /// Recursively propagates failed state up the dependency tree
    fn propagate_failed_state(
        infos: &mut AssetInfos,
        failed_id: ErasedAssetIndex,
        waiting_id: ErasedAssetIndex,
        error: &Arc<AssetLoadError>,
    ) {
        let dependents_waiting_on_rec_load = if let Some(info) = infos.get_mut(waiting_id) {
            info.loading_rec_dependencies.remove(&failed_id);
            info.failed_rec_dependencies.insert(failed_id);
            info.rec_dep_load_state = RecursiveDependencyLoadState::Failed(error.clone());
            Some(core::mem::take(
                &mut info.dependents_waiting_on_recursive_dep_load,
            ))
        } else {
            None
        };

        if let Some(dependents_waiting_on_rec_load) = dependents_waiting_on_rec_load {
            for dep_id in dependents_waiting_on_rec_load {
                Self::propagate_failed_state(infos, waiting_id, dep_id, error);
            }
        }
    }

    pub(crate) fn process_asset_fail(
        &mut self,
        failed_index: ErasedAssetIndex,
        error: AssetLoadError,
    ) {
        // Check whether the handle has been dropped since the asset was loaded.
        if !self.infos.contains_key(&failed_index) {
            return;
        }

        let error = Arc::new(error);
        let (dependents_waiting_on_load, dependents_waiting_on_rec_load) = {
            let Some(info) = self.get_mut(failed_index) else {
                // The asset was already dropped.
                return;
            };
            info.load_state = LoadState::Failed(error.clone());
            info.dep_load_state = DependencyLoadState::Failed(error.clone());
            info.rec_dep_load_state = RecursiveDependencyLoadState::Failed(error.clone());
            for waker in info.waiting_tasks.drain(..) {
                waker.wake();
            }
            (
                core::mem::take(&mut info.dependents_waiting_on_load),
                core::mem::take(&mut info.dependents_waiting_on_recursive_dep_load),
            )
        };

        for waiting_id in dependents_waiting_on_load {
            if let Some(info) = self.get_mut(waiting_id) {
                info.loading_dependencies.remove(&failed_index);
                info.failed_dependencies.insert(failed_index);
                // don't overwrite DependencyLoadState if already failed to preserve first error
                if !info.dep_load_state.is_failed() {
                    info.dep_load_state = DependencyLoadState::Failed(error.clone());
                }
            }
        }

        for waiting_id in dependents_waiting_on_rec_load {
            Self::propagate_failed_state(self, failed_index, waiting_id, &error);
        }
    }

    fn remove_dependents_and_labels(
        info: &AssetInfo,
        loader_dependents: &mut HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
        path: &AssetPath<'static>,
        living_labeled_assets: &mut HashMap<AssetPath<'static>, HashSet<Box<str>>>,
    ) {
        for loader_dependency in info.loader_dependencies.keys() {
            if let Some(dependents) = loader_dependents.get_mut(loader_dependency) {
                dependents.remove(path);
            }
        }

        let Some(label) = path.label() else {
            return;
        };

        let mut without_label = path.to_owned();
        without_label.remove_label();

        let Entry::Occupied(mut entry) = living_labeled_assets.entry(without_label) else {
            return;
        };

        entry.get_mut().remove(label);
        if entry.get().is_empty() {
            entry.remove();
        }
    }

    fn process_asset_entry_drop_internal(
        infos: &mut HashMap<ErasedAssetIndex, AssetInfo>,
        path_to_id: &mut HashMap<AssetPath<'static>, TypeIdMap<AssetIndex>>,
        loader_dependents: &mut HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
        living_labeled_assets: &mut HashMap<AssetPath<'static>, HashSet<Box<str>>>,
        pending_tasks: &mut HashMap<ErasedAssetIndex, Task<()>>,
        watching_for_changes: bool,
        index: ErasedAssetIndex,
    ) {
        let Entry::Occupied(entry) = infos.entry(index) else {
            // Either the asset was already dropped, it doesn't exist, or it isn't managed by the
            // asset server. In any of these cases, there's nothing more to do.
            return;
        };

        pending_tasks.remove(&index);

        let type_id = entry.key().type_id;

        let info = entry.remove();
        let Some(path) = &info.path else {
            return;
        };

        if watching_for_changes {
            Self::remove_dependents_and_labels(
                &info,
                loader_dependents,
                path,
                living_labeled_assets,
            );
        }

        if let Some(map) = path_to_id.get_mut(path) {
            map.remove(&type_id);

            if map.is_empty() {
                path_to_id.remove(path);
            }
        };
    }

    /// Consumes all current handle drop events. This will update information in [`AssetInfos`], but it
    /// will not affect [`Assets`] storages. For normal use cases, prefer `Assets::track_assets()`
    /// This should only be called if `Assets` storage isn't being used (such as in [`AssetProcessor`](crate::processor::AssetProcessor))
    ///
    /// [`Assets`]: crate::Assets
    pub(crate) fn consume_handle_drop_events(&mut self) {
        for provider in self.handle_providers.values() {
            while let Ok(handle_event) = provider.event_receiver.try_recv() {
                match handle_event {
                    HandleEvent::New(_) => {}
                    HandleEvent::Drop(index) => {
                        Self::process_asset_entry_drop_internal(
                            &mut self.infos,
                            &mut self.path_to_index,
                            &mut self.loader_dependents,
                            &mut self.living_labeled_assets,
                            &mut self.pending_tasks,
                            self.watching_for_changes,
                            ErasedAssetIndex {
                                index,
                                type_id: provider.type_id,
                            },
                        );
                    }
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

/// An error encountered during [`AssetInfos::get_or_create_path_handle_internal`].
#[derive(Error, Debug)]
pub(crate) enum GetOrCreateHandleInternalError {
    #[error(transparent)]
    MissingHandleProviderError(#[from] MissingHandleProviderError),
    #[error("Handle does not exist but TypeId was not specified.")]
    HandleMissingButTypeIdNotSpecified,
}

pub(crate) fn unwrap_with_context<T>(
    result: Result<T, GetOrCreateHandleInternalError>,
    type_info: Either<&str, TypeId>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(GetOrCreateHandleInternalError::HandleMissingButTypeIdNotSpecified) => None,
        Err(GetOrCreateHandleInternalError::MissingHandleProviderError(_)) => match type_info {
            Either::Left(type_name) => {
                panic!("Cannot allocate an Asset Handle of type '{type_name}' because the asset type has not been initialized. \
                    Make sure you have called `app.init_asset::<{type_name}>()`");
            }
            Either::Right(type_id) => {
                panic!("Cannot allocate an AssetHandle of type '{type_id:?}' because the asset type has not been initialized. \
                    Make sure you have called `app.init_asset::<(actual asset type)>()`")
            }
        },
    }
}
