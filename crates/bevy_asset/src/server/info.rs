use crate::{
    meta::{AssetHash, MetaTransform},
    Asset, AssetEntity, AssetHandleProvider, AssetLoadError, AssetPath, DependencyLoadState,
    ErasedLoadedAsset, Handle, InternalAssetEvent, LoadState, RecursiveDependencyLoadState,
    StrongHandle, UntypedHandle,
};
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    sync::{Arc, Weak},
    vec::Vec,
};
use bevy_ecs::{entity::Entity, world::World};
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_tasks::Task;
use bevy_utils::TypeIdMap;
use core::{any::TypeId, task::Waker};
use crossbeam_channel::Sender;
use thiserror::Error;
use tracing::warn;

#[derive(Debug)]
pub(crate) struct AssetInfo {
    /// The ID of this asset.
    type_id: TypeId,
    /// A non-owning handle to the asset.
    weak_handle: Weak<StrongHandle>,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) load_state: LoadState,
    pub(crate) dep_load_state: DependencyLoadState,
    pub(crate) rec_dep_load_state: RecursiveDependencyLoadState,
    loading_dependencies: HashSet<AssetEntity>,
    failed_dependencies: HashSet<AssetEntity>,
    loading_rec_dependencies: HashSet<AssetEntity>,
    failed_rec_dependencies: HashSet<AssetEntity>,
    dependents_waiting_on_load: HashSet<AssetEntity>,
    dependents_waiting_on_recursive_dep_load: HashSet<AssetEntity>,
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
    fn new(
        type_id: TypeId,
        weak_handle: Weak<StrongHandle>,
        path: Option<AssetPath<'static>>,
    ) -> Self {
        Self {
            type_id,
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

pub(crate) struct AssetInfos {
    path_to_entity: HashMap<AssetPath<'static>, TypeIdMap<AssetEntity>>,
    infos: HashMap<AssetEntity, AssetInfo>,
    /// If set to `true`, this informs [`AssetInfos`] to track data relevant to watching for changes (such as `load_dependents`)
    /// This should only be set at startup.
    pub(crate) watching_for_changes: bool,
    /// Tracks assets that depend on the "key" asset path inside their asset loaders ("loader dependencies")
    /// This should only be set when watching for changes to avoid unnecessary work.
    pub(crate) loader_dependents: HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
    /// Tracks living labeled assets for a given source asset.
    /// This should only be set when watching for changes to avoid unnecessary work.
    pub(crate) living_labeled_assets: HashMap<AssetPath<'static>, HashSet<Box<str>>>,
    pub(crate) handle_provider: AssetHandleProvider,
    pub(crate) dependency_loaded_event_sender: TypeIdMap<fn(&mut World, AssetEntity)>,
    pub(crate) dependency_failed_event_sender:
        TypeIdMap<fn(&mut World, AssetEntity, AssetPath<'static>, AssetLoadError)>,
    pub(crate) pending_tasks: HashMap<AssetEntity, Task<()>>,
    /// The stats that have collected during usage of the asset server.
    pub(crate) stats: AssetServerStats,
}

impl core::fmt::Debug for AssetInfos {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AssetInfos")
            .field("path_to_index", &self.path_to_entity)
            .field("infos", &self.infos)
            .finish()
    }
}

impl AssetInfos {
    pub(crate) fn new(handle_provider: AssetHandleProvider) -> Self {
        Self {
            handle_provider,
            path_to_entity: Default::default(),
            infos: Default::default(),
            watching_for_changes: Default::default(),
            loader_dependents: Default::default(),
            living_labeled_assets: Default::default(),
            dependency_loaded_event_sender: Default::default(),
            dependency_failed_event_sender: Default::default(),
            pending_tasks: Default::default(),
            stats: Default::default(),
        }
    }

    pub(crate) fn create_loading_handle_untyped(
        &mut self,
        type_id: TypeId,
        builder: &mut impl HandleBuilder,
    ) -> Arc<StrongHandle> {
        let entity = AssetEntity::new_unchecked(builder.create_entity());
        let handle = Self::create_handle_internal(
            entity,
            &mut self.infos,
            &self.handle_provider,
            &mut self.living_labeled_assets,
            self.watching_for_changes,
            type_id,
            None,
            None,
            true,
        );
        builder.trigger_setup(&handle);
        handle
    }

    fn create_handle_internal(
        entity: AssetEntity,
        infos: &mut HashMap<AssetEntity, AssetInfo>,
        handle_provider: &AssetHandleProvider,
        living_labeled_assets: &mut HashMap<AssetPath<'static>, HashSet<Box<str>>>,
        watching_for_changes: bool,
        type_id: TypeId,
        path: Option<AssetPath<'static>>,
        meta_transform: Option<MetaTransform>,
        loading: bool,
    ) -> Arc<StrongHandle> {
        if watching_for_changes && let Some(path) = &path {
            let mut without_label = path.to_owned();
            if let Some(label) = without_label.take_label() {
                let labels = living_labeled_assets.entry(without_label).or_default();
                labels.insert(label.as_ref().into());
            }
        }

        let handle = handle_provider.create_handle(entity, type_id, path.clone(), meta_transform);
        let mut info = AssetInfo::new(type_id, Arc::downgrade(&handle), path);
        if loading {
            info.load_state = LoadState::Loading;
            info.dep_load_state = DependencyLoadState::Loading;
            info.rec_dep_load_state = RecursiveDependencyLoadState::Loading;
        }
        infos.insert(handle.entity, info);

        handle
    }

    pub(crate) fn get_or_create_path_handle<A: Asset>(
        &mut self,
        path: AssetPath<'static>,
        loading_mode: HandleLoadingMode,
        meta_transform: Option<MetaTransform>,
        builder: &mut impl HandleBuilder,
    ) -> (Handle<A>, bool) {
        let (handle, should_load) = self
            .get_or_create_path_handle_internal(
                path,
                Some(TypeId::of::<A>()),
                loading_mode,
                meta_transform,
                builder,
            )
            .expect("we specified the TypeId");
        (handle.typed_unchecked(), should_load)
    }

    pub(crate) fn get_or_create_path_handle_erased(
        &mut self,
        path: AssetPath<'static>,
        type_id: TypeId,
        loading_mode: HandleLoadingMode,
        meta_transform: Option<MetaTransform>,
        builder: &mut impl HandleBuilder,
    ) -> (UntypedHandle, bool) {
        self.get_or_create_path_handle_internal(
            path,
            Some(type_id),
            loading_mode,
            meta_transform,
            builder,
        )
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
        builder: &mut impl HandleBuilder,
    ) -> Result<(UntypedHandle, bool), GetOrCreateHandleInternalError> {
        let handles = self.path_to_entity.entry(path.clone()).or_default();

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

        // Note: Pass in `infos` here so we can borrow it later on.
        let create_new_handle = |infos: &mut HashMap<AssetEntity, AssetInfo>| {
            let should_load = match loading_mode {
                HandleLoadingMode::NotLoading => false,
                HandleLoadingMode::Request | HandleLoadingMode::Force => true,
            };
            let entity = AssetEntity::new_unchecked(builder.create_entity());
            let handle = AssetInfos::create_handle_internal(
                entity,
                infos,
                &self.handle_provider,
                &mut self.living_labeled_assets,
                self.watching_for_changes,
                type_id,
                Some(path),
                meta_transform,
                should_load,
            );
            builder.trigger_setup(&handle);
            (handle, should_load, entity)
        };

        match handles.entry(type_id) {
            Entry::Occupied(mut entry) => {
                let entity = *entry.get();
                // if there is a path_to_entity entry, info always exists
                let info = self.infos.get_mut(&entity).unwrap();

                if let Some(strong_handle) = info.weak_handle.upgrade() {
                    // If we can upgrade the handle, there is at least one live handle right now,
                    // The asset load has already kicked off (and maybe completed), so we can just
                    // return a strong handle

                    let mut should_load = false;
                    if loading_mode == HandleLoadingMode::Force
                        || (loading_mode == HandleLoadingMode::Request
                            && matches!(
                                info.load_state,
                                LoadState::NotLoaded | LoadState::Failed(_)
                            ))
                    {
                        info.load_state = LoadState::Loading;
                        info.dep_load_state = DependencyLoadState::Loading;
                        info.rec_dep_load_state = RecursiveDependencyLoadState::Loading;
                        should_load = true;
                    }
                    Ok((UntypedHandle::Strong(strong_handle), should_load))
                } else {
                    // Asset meta exists, but all live handles were dropped. This means the
                    // `despawn_unused_assets` system hasn't been run yet to remove the current
                    // asset. We must create a new entity and handle for this path.
                    let (handle, should_load, entity) = create_new_handle(&mut self.infos);
                    entry.insert(entity);
                    Ok((UntypedHandle::Strong(handle), should_load))
                }
            }
            // The entry does not exist, so this is a "fresh" asset load. We must create a new handle
            Entry::Vacant(entry) => {
                let (handle, should_load, entity) = create_new_handle(&mut self.infos);
                entry.insert(entity);
                Ok((UntypedHandle::Strong(handle), should_load))
            }
        }
    }

    pub(crate) fn get(&self, entity: AssetEntity) -> Option<&AssetInfo> {
        self.infos.get(&entity)
    }

    pub(crate) fn contains_key(&self, entity: AssetEntity) -> bool {
        self.infos.contains_key(&entity)
    }

    pub(crate) fn get_mut(&mut self, entity: AssetEntity) -> Option<&mut AssetInfo> {
        self.infos.get_mut(&entity)
    }

    pub(crate) fn get_path_and_type_id_handle(
        &self,
        path: &AssetPath<'_>,
        type_id: TypeId,
    ) -> Option<UntypedHandle> {
        let entity = *self.path_to_entity.get(path)?.get(&type_id)?;
        self.get_index_handle(entity)
    }

    pub(crate) fn get_path_entities<'a>(
        &'a self,
        path: &'a AssetPath<'_>,
    ) -> impl Iterator<Item = AssetEntity> + 'a {
        /// Concrete type to allow returning an `impl Iterator` even if `self.path_to_id.get(&path)` is `None`
        enum HandlesByPathIterator<T> {
            None,
            Some(T),
        }

        impl<T> Iterator for HandlesByPathIterator<T>
        where
            T: Iterator,
        {
            type Item = T::Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    HandlesByPathIterator::None => None,
                    HandlesByPathIterator::Some(iter) => iter.next(),
                }
            }
        }

        if let Some(type_id_to_entity) = self.path_to_entity.get(path) {
            HandlesByPathIterator::Some(type_id_to_entity.values().cloned())
        } else {
            HandlesByPathIterator::None
        }
    }

    pub(crate) fn get_path_handles<'a>(
        &'a self,
        path: &'a AssetPath<'_>,
    ) -> impl Iterator<Item = UntypedHandle> + 'a {
        self.get_path_entities(path)
            .filter_map(|id| self.get_index_handle(id))
    }

    pub(crate) fn get_index_handle(&self, entity: AssetEntity) -> Option<UntypedHandle> {
        let info = self.infos.get(&entity)?;
        let strong_handle = info.weak_handle.upgrade()?;
        Some(UntypedHandle::Strong(strong_handle))
    }

    /// Returns `true` if the asset this path points to is still alive
    pub(crate) fn is_path_alive<'a>(&self, path: impl Into<AssetPath<'a>>) -> bool {
        self.get_path_entities(&path.into())
            .filter_map(|id| self.infos.get(&id))
            .any(|info| info.weak_handle.strong_count() > 0)
    }

    /// Returns `true` if the asset at this path should be reloaded
    pub(crate) fn should_reload(&self, path: &AssetPath) -> bool {
        if self.is_path_alive(path) {
            return true;
        }

        if let Some(living) = self.living_labeled_assets.get(path) {
            !living.is_empty()
        } else {
            false
        }
    }

    /// Returns `true` if the asset should be removed from the collection.
    pub(crate) fn process_handle_drop(&mut self, entity: AssetEntity) {
        Self::process_handle_drop_internal(
            &mut self.infos,
            &mut self.path_to_entity,
            &mut self.loader_dependents,
            &mut self.living_labeled_assets,
            &mut self.pending_tasks,
            self.watching_for_changes,
            entity,
        );
    }

    /// Updates [`AssetInfo`] / load state for an asset that has finished loading (and relevant dependencies / dependents).
    pub(crate) fn process_asset_load(
        &mut self,
        entity: AssetEntity,
        loaded_asset: ErasedLoadedAsset,
        world: &mut World,
        sender: &Sender<InternalAssetEvent>,
    ) {
        // Process all the labeled assets first so that they don't get skipped due to the "parent"
        // not having its handle alive.
        for asset in loaded_asset.labeled_assets {
            let UntypedHandle::Strong(handle) = &asset.handle else {
                unreachable!("Labeled assets are always strong handles");
            };
            self.process_asset_load(handle.entity, asset.asset, world, sender);
        }

        // Check whether the handle has been dropped since the asset was loaded.
        if !self.infos.contains_key(&entity) {
            return;
        }

        let asset_type_id = loaded_asset.value.type_id();
        // This should be impossible, since we still have the asset server metadata, which means the
        // metadata hasn't been removed by `AssetServerManaged`s hook.
        loaded_asset
            .value
            .insert(entity, world)
            .expect("asset metadata still exists in AssetServer");
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
                            .insert(entity);
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
                        dep_info.dependents_waiting_on_load.insert(entity);
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
                    dep_id, entity
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
                        entity,
                        type_id: asset_type_id,
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
                    .get(&entity)
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
                .get_mut(entity)
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
                info.loading_dependencies.remove(&entity);
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
                        Self::propagate_loaded_state(self, entity, dep_id, sender);
                    }
                }
                RecursiveDependencyLoadState::Failed(ref error) => {
                    for dep_id in dependents_waiting_on_rec_load {
                        Self::propagate_failed_state(self, entity, dep_id, error);
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
        loaded_entity: AssetEntity,
        waiting_entity: AssetEntity,
        sender: &Sender<InternalAssetEvent>,
    ) {
        let dependents_waiting_on_rec_load = if let Some(info) = infos.get_mut(waiting_entity) {
            info.loading_rec_dependencies.remove(&loaded_entity);
            if info.loading_rec_dependencies.is_empty() && info.failed_rec_dependencies.is_empty() {
                info.rec_dep_load_state = RecursiveDependencyLoadState::Loaded;
                if info.load_state.is_loaded() {
                    sender
                        .send(InternalAssetEvent::LoadedWithDependencies {
                            entity: waiting_entity,
                            type_id: info.type_id,
                        })
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
                Self::propagate_loaded_state(infos, waiting_entity, dep_id, sender);
            }
        }
    }

    /// Recursively propagates failed state up the dependency tree
    fn propagate_failed_state(
        infos: &mut AssetInfos,
        failed_entity: AssetEntity,
        waiting_entity: AssetEntity,
        error: &Arc<AssetLoadError>,
    ) {
        let dependents_waiting_on_rec_load = if let Some(info) = infos.get_mut(waiting_entity) {
            info.loading_rec_dependencies.remove(&failed_entity);
            info.failed_rec_dependencies.insert(failed_entity);
            info.rec_dep_load_state = RecursiveDependencyLoadState::Failed(error.clone());
            Some(core::mem::take(
                &mut info.dependents_waiting_on_recursive_dep_load,
            ))
        } else {
            None
        };

        if let Some(dependents_waiting_on_rec_load) = dependents_waiting_on_rec_load {
            for dep_id in dependents_waiting_on_rec_load {
                Self::propagate_failed_state(infos, waiting_entity, dep_id, error);
            }
        }
    }

    pub(crate) fn process_asset_fail(&mut self, failed_entity: AssetEntity, error: AssetLoadError) {
        // Check whether the handle has been dropped since the asset was loaded.
        if !self.infos.contains_key(&failed_entity) {
            return;
        }

        let error = Arc::new(error);
        let (dependents_waiting_on_load, dependents_waiting_on_rec_load) = {
            let Some(info) = self.get_mut(failed_entity) else {
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

        for waiting_entity in dependents_waiting_on_load {
            if let Some(info) = self.get_mut(waiting_entity) {
                info.loading_dependencies.remove(&failed_entity);
                info.failed_dependencies.insert(failed_entity);
                // don't overwrite DependencyLoadState if already failed to preserve first error
                if !info.dep_load_state.is_failed() {
                    info.dep_load_state = DependencyLoadState::Failed(error.clone());
                }
            }
        }

        for waiting_entity in dependents_waiting_on_rec_load {
            Self::propagate_failed_state(self, failed_entity, waiting_entity, &error);
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

    fn process_handle_drop_internal(
        infos: &mut HashMap<AssetEntity, AssetInfo>,
        path_to_entity: &mut HashMap<AssetPath<'static>, TypeIdMap<AssetEntity>>,
        loader_dependents: &mut HashMap<AssetPath<'static>, HashSet<AssetPath<'static>>>,
        living_labeled_assets: &mut HashMap<AssetPath<'static>, HashSet<Box<str>>>,
        pending_tasks: &mut HashMap<AssetEntity, Task<()>>,
        watching_for_changes: bool,
        entity: AssetEntity,
    ) {
        let Entry::Occupied(entry) = infos.entry(entity) else {
            // Either the asset was already dropped, it doesn't exist, or it isn't managed by the asset server
            // None of these cases should result in a removal from the Assets collection
            return;
        };

        pending_tasks.remove(&entity);

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

        // Try to remove the entity from `path_to_entity`.
        if let Some(map) = path_to_entity.get_mut(path)
            && let Entry::Occupied(entry) = map.entry(info.type_id)
            // Make sure that `entity` is still the most "up-to-date" entity for this path. It may
            // not be if this entity's handle was dropped, then another load occurred, and then that
            // new entity was manually despawned. Very unlikely, but we don't need to do anything in
            // that case.
            && *entry.get() == entity
        {
            entry.remove();
            if map.is_empty() {
                path_to_entity.remove(path);
            }
        };
    }

    /// Consumes all current handle drop events. This will update information in [`AssetInfos`], but it
    /// will not affect asset entities. For normal use cases, prefer [`despawn_unused_assets`](crate::despawn_unused_assets).
    /// This should only be called if asset entities aren't being used (such as in [`AssetProcessor`](crate::processor::AssetProcessor))
    pub(crate) fn consume_handle_drop_events(&mut self) {
        while let Ok((entity, _)) = self.handle_provider.drop_receiver.try_recv() {
            Self::process_handle_drop_internal(
                &mut self.infos,
                &mut self.path_to_entity,
                &mut self.loader_dependents,
                &mut self.living_labeled_assets,
                &mut self.pending_tasks,
                self.watching_for_changes,
                entity,
            );
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

/// An (internal) trait for creating handles.
pub(crate) trait HandleBuilder {
    /// Creates the entity that will be used for the handle.
    fn create_entity(&mut self) -> Entity;

    /// Invokes the setup of the asset for the newly created handle.
    fn trigger_setup(&mut self, handle: &Arc<StrongHandle>);
}

/// An error encountered during [`AssetInfos::get_or_create_path_handle_internal`].
#[derive(Error, Debug)]
pub(crate) enum GetOrCreateHandleInternalError {
    #[error("Handle does not exist but TypeId was not specified.")]
    HandleMissingButTypeIdNotSpecified,
}
