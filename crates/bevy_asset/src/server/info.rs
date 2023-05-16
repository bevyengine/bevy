use crate::{
    AssetHandleProvider, AssetPath, DependencyLoadState, ErasedLoadedAsset, InternalAssetEvent,
    InternalAssetHandle, LoadState, RecursiveDependencyLoadState, UntypedAssetId, UntypedHandle,
};
use bevy_ecs::world::World;
use bevy_log::warn;
use bevy_utils::{Entry, HashMap, HashSet};
use crossbeam_channel::Sender;
use std::{
    any::TypeId,
    sync::{Arc, Weak},
};

#[derive(Debug)]
pub(crate) struct AssetInfo {
    weak_handle: Weak<InternalAssetHandle>,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) load_state: LoadState,
    pub(crate) dep_load_state: DependencyLoadState,
    pub(crate) rec_dep_load_state: RecursiveDependencyLoadState,
    loading_dependencies: usize,
    failed_dependencies: usize,
    loading_rec_dependencies: usize,
    failed_rec_dependencies: usize,
    dependants_waiting_on_load: HashSet<UntypedAssetId>,
    dependants_waiting_on_recursive_dep_load: HashSet<UntypedAssetId>,
    handle_drops_to_skip: usize,
}

impl AssetInfo {
    fn new(weak_handle: Weak<InternalAssetHandle>, path: Option<AssetPath<'static>>) -> Self {
        Self {
            weak_handle,
            path,
            load_state: LoadState::NotLoaded,
            dep_load_state: DependencyLoadState::NotLoaded,
            rec_dep_load_state: RecursiveDependencyLoadState::NotLoaded,
            loading_dependencies: 0,
            failed_dependencies: 0,
            loading_rec_dependencies: 0,
            failed_rec_dependencies: 0,
            dependants_waiting_on_load: HashSet::default(),
            dependants_waiting_on_recursive_dep_load: HashSet::default(),
            handle_drops_to_skip: 0,
        }
    }
}

#[derive(Default)]
pub struct AssetInfos {
    path_to_id: HashMap<AssetPath<'static>, UntypedAssetId>,
    infos: HashMap<UntypedAssetId, AssetInfo>,
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
    pub(crate) fn create_loading_handle(&mut self, type_id: TypeId) -> UntypedHandle {
        Self::create_handle_internal(&mut self.infos, &self.handle_providers, type_id, None, true)
    }

    fn create_handle_internal(
        infos: &mut HashMap<UntypedAssetId, AssetInfo>,
        handle_providers: &HashMap<TypeId, AssetHandleProvider>,
        type_id: TypeId,
        path: Option<AssetPath<'static>>,
        loading: bool,
    ) -> UntypedHandle {
        let provider = handle_providers.get(&type_id).unwrap_or_else(|| {
            panic!(
                "Cannot allocate a handle for asset of type {:?} because it does not exist",
                type_id
            )
        });

        let handle = provider.reserve_handle_internal(true, path.clone());
        let mut info = AssetInfo::new(Arc::downgrade(&handle), path);
        if loading {
            info.load_state = LoadState::Loading;
            info.dep_load_state = DependencyLoadState::Loading;
            info.rec_dep_load_state = RecursiveDependencyLoadState::Loading;
        }
        infos.insert(handle.id, info);
        UntypedHandle::Strong(handle)
    }

    /// Retrieves asset tracking data, or creates it if it doesn't exist.
    /// Returns true if an asset load should be kicked off
    pub(crate) fn get_or_create_path_handle(
        &mut self,
        path: AssetPath<'static>,
        type_id: TypeId,
        loading_mode: HandleLoadingMode,
    ) -> (UntypedHandle, bool) {
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
                    (UntypedHandle::Strong(strong_handle), should_load)
                } else {
                    // Asset meta exists, but all live handles were dropped. This means the `track_assets` system
                    // hasn't been run yet to remove the current asset
                    // (note that this is guaranteed to be transactional with the `track_assets` system because
                    // because it locks the AssetInfos collection)

                    // We must create a new strong handle for the existing id and ensure that the drop of the old
                    // strong handle doesn't remove the asset from the Assets collection
                    info.handle_drops_to_skip += 1;
                    let provider = self.handle_providers.get(&type_id).unwrap_or_else(|| {
                        panic!(
                            "Cannot allocate a handle for asset of type {:?} because it does not exist",
                            type_id
                        )
                    });
                    let handle = provider.get_handle(id.internal(), true, Some(path));
                    info.weak_handle = Arc::downgrade(&handle);
                    (UntypedHandle::Strong(handle), should_load)
                }
            }
            // The entry does not exist, so this is a "fresh" asset load. We must create a new handle
            Entry::Vacant(entry) => {
                let should_load = match loading_mode {
                    HandleLoadingMode::NotLoading => false,
                    HandleLoadingMode::Request => true,
                    HandleLoadingMode::Force => true,
                };
                let handle = Self::create_handle_internal(
                    &mut self.infos,
                    &self.handle_providers,
                    type_id,
                    Some(path),
                    should_load,
                );
                entry.insert(handle.id());
                (handle, should_load)
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
        let info = self.infos.get(&id)?;
        let strong_handle = info.weak_handle.upgrade()?;
        Some(UntypedHandle::Strong(strong_handle))
    }

    /// Returns `true` if this path has
    pub(crate) fn is_path_alive(&self, path: &AssetPath) -> bool {
        if let Some(id) = self.path_to_id.get(path) {
            if let Some(info) = self.infos.get(id) {
                return info.weak_handle.strong_count() > 0;
            }
        }
        false
    }

    // Returns `true` if the asset should be removed from the collection
    pub(crate) fn process_handle_drop(&mut self, id: UntypedAssetId) -> bool {
        Self::process_handle_drop_internal(&mut self.infos, &mut self.path_to_id, id)
    }

    pub(crate) fn process_asset_load(
        &mut self,
        loaded_asset_id: UntypedAssetId,
        loaded_asset: ErasedLoadedAsset,
        world: &mut World,
        sender: &Sender<InternalAssetEvent>,
    ) {
        loaded_asset.value.insert(loaded_asset_id, world);
        let mut loading_deps = loaded_asset.dependencies.len();
        let mut failed_deps = 0;
        let mut loading_rec_deps = loaded_asset.dependencies.len();
        let mut failed_rec_deps = 0;
        for dep_id in loaded_asset.dependencies.iter() {
            if let Some(dep_info) = self.get_mut(dep_id.id()) {
                match dep_info.load_state {
                    LoadState::NotLoaded | LoadState::Loading => {
                        // If dependency is loading, wait for it.
                        dep_info.dependants_waiting_on_load.insert(loaded_asset_id);
                    }
                    LoadState::Loaded => {
                        // If dependency is loaded, reduce our count by one
                        loading_deps -= 1;
                    }
                    LoadState::Failed => {
                        failed_deps += 1;
                        loading_deps -= 1;
                    }
                }
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
                        loading_rec_deps -= 1;
                    }
                    RecursiveDependencyLoadState::Failed => {
                        failed_rec_deps += 1;
                        loading_rec_deps -= 1;
                    }
                }
            } else {
                // the dependency id does not exist, which implies it was manually removed or never existed in the first place
                warn!(
                    "Dependency {:?} from asset {:?} is unknown. This asset's dependency load status will not switch to 'Loaded' until the unknown dependency is loaded.",
                    dep_id, loaded_asset_id
                );
            }
        }

        let dep_load_state = match (loading_deps, failed_deps) {
            (0, 0) => DependencyLoadState::Loaded,
            (_loading, 0) => DependencyLoadState::Loading,
            (_loading, _failed) => DependencyLoadState::Failed,
        };

        let rec_dep_load_state = match (loading_rec_deps, failed_rec_deps) {
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
                info.loading_dependencies -= 1;
                if info.loading_dependencies == 0 {
                    // send dependencies loaded event
                    info.dep_load_state = DependencyLoadState::Loaded;
                }
            }
        }

        if let Some(dependants_waiting_on_rec_load) = dependants_waiting_on_rec_load {
            match rec_dep_load_state {
                RecursiveDependencyLoadState::Loaded => {
                    for dep_id in dependants_waiting_on_rec_load {
                        Self::propagate_loaded_state(self, dep_id, sender);
                    }
                }
                RecursiveDependencyLoadState::Failed => {
                    for dep_id in dependants_waiting_on_rec_load {
                        Self::propagate_failed_state(self, dep_id);
                    }
                }
                RecursiveDependencyLoadState::Loading | RecursiveDependencyLoadState::NotLoaded => {
                    // dependants_waiting_on_rec_load should be None in this case
                    unreachable!("`Loading` and `NotLoaded` state should never be propagated.")
                }
            }
        }
    }

    fn propagate_loaded_state(
        infos: &mut AssetInfos,
        id: UntypedAssetId,
        sender: &Sender<InternalAssetEvent>,
    ) {
        let dependants_waiting_on_rec_load = if let Some(info) = infos.get_mut(id) {
            info.loading_rec_dependencies -= 1;
            if info.loading_rec_dependencies == 0 && info.failed_rec_dependencies == 0 {
                info.rec_dep_load_state = RecursiveDependencyLoadState::Loaded;
                if info.load_state == LoadState::Loaded {
                    sender
                        .send(InternalAssetEvent::LoadedWithDependencies { id })
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
                Self::propagate_loaded_state(infos, dep_id, sender);
            }
        }
    }

    fn propagate_failed_state(infos: &mut AssetInfos, id: UntypedAssetId) {
        let dependants_waiting_on_rec_load = if let Some(info) = infos.get_mut(id) {
            info.loading_rec_dependencies -= 1;
            info.failed_rec_dependencies += 1;
            info.rec_dep_load_state = RecursiveDependencyLoadState::Failed;
            Some(std::mem::take(
                &mut info.dependants_waiting_on_recursive_dep_load,
            ))
        } else {
            None
        };

        if let Some(dependants_waiting_on_rec_load) = dependants_waiting_on_rec_load {
            for dep_id in dependants_waiting_on_rec_load {
                Self::propagate_failed_state(infos, dep_id);
            }
        }
    }

    pub(crate) fn process_asset_fail(&mut self, id: UntypedAssetId) {
        let (dependants_waiting_on_load, dependants_waiting_on_rec_load) = {
            let info = self
                .get_mut(id)
                .expect("Asset info should always exist at this point");
            info.load_state = LoadState::Failed;
            info.dep_load_state = DependencyLoadState::Failed;
            info.rec_dep_load_state = RecursiveDependencyLoadState::Failed;
            (
                std::mem::take(&mut info.dependants_waiting_on_load),
                std::mem::take(&mut info.dependants_waiting_on_recursive_dep_load),
            )
        };

        for id in dependants_waiting_on_load {
            if let Some(info) = self.get_mut(id) {
                info.loading_dependencies -= 1;
                info.dep_load_state = DependencyLoadState::Failed;
            }
        }

        for dep_id in dependants_waiting_on_rec_load {
            Self::propagate_failed_state(self, dep_id);
        }
    }

    fn process_handle_drop_internal(
        infos: &mut HashMap<UntypedAssetId, AssetInfo>,
        path_to_id: &mut HashMap<AssetPath<'static>, UntypedAssetId>,
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

    /// Consumes all current handle drop events. This will update information in AssetInfos, but it
    /// will not affect [`Assets`] storages. For normal use cases, prefer `Assets::track_assets()`
    /// This should only be called if `Assets` storage isn't being used (such as in [`AssetProcessor`](crate::processor::AssetProcessor))
    pub(crate) fn consume_handle_drop_events(&mut self) {
        for provider in self.handle_providers.values() {
            while let Ok(drop_event) = provider.drop_receiver.try_recv() {
                let id = drop_event.id;
                if drop_event.asset_server_managed {
                    Self::process_handle_drop_internal(
                        &mut self.infos,
                        &mut self.path_to_id,
                        id.untyped(provider.type_id),
                    );
                }
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum HandleLoadingMode {
    NotLoading,
    Request,
    Force,
}
