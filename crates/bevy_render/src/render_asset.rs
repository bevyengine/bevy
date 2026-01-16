use crate::{
    render_resource::AsBindGroupError, Extract, ExtractSchedule, MainWorld, Render, RenderApp,
    RenderSystems, Res,
};
use alloc::collections::BTreeMap;
use bevy_app::{App, Plugin, SubApp};
use bevy_asset::{
    Asset, AssetEvent, AssetId, Assets, RenderAssetTransferPriority, RenderAssetUsages,
};
use bevy_ecs::{
    prelude::{Commands, IntoScheduleConfigs, MessageReader, ResMut, Resource},
    schedule::SystemSet,
    system::{StaticSystemParam, SystemParam, SystemParamItem, SystemState},
    world::{FromWorld, Mut},
};
use bevy_platform::collections::{HashMap, HashSet};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Error)]
pub enum PrepareAssetError<E: Send + Sync + 'static> {
    #[error("Failed to prepare asset")]
    RetryNextUpdate(E),
    #[error("Failed to build bind group: {0}")]
    AsBindGroupError(AsBindGroupError),
}

/// The system set during which we extract modified assets to the render world.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct AssetExtractionSystems;

/// Error returned when an asset due for extraction has already been extracted
#[derive(Debug, Error)]
pub enum AssetExtractionError {
    #[error("The asset has already been extracted")]
    AlreadyExtracted,
    #[error("The asset type does not support extraction. To clone the asset to the renderworld, use `RenderAssetUsages::default()`")]
    NoExtractionImplementation,
}

/// Describes how an asset gets extracted and prepared for rendering.
///
/// In the [`ExtractSchedule`] step the [`RenderAsset::SourceAsset`] is transferred
/// from the "main world" into the "render world".
///
/// After that in the [`RenderSystems::PrepareAssets`] step the extracted asset
/// is transformed into its GPU-representation of type [`RenderAsset`].
pub trait RenderAsset: Send + Sync + 'static + Sized {
    /// The representation of the asset in the "main world".
    type SourceAsset: Asset + Clone;

    /// Specifies all ECS data required by [`RenderAsset::prepare_asset`].
    ///
    /// For convenience use the [`lifetimeless`](bevy_ecs::system::lifetimeless) [`SystemParam`].
    type Param: SystemParam;

    /// Whether or not to unload the asset after extracting it to the render world.
    #[inline]
    fn asset_usage(_source_asset: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::default()
    }

    /// Priority for GPU transfer, and optionally size of the data the asset will upload to the gpu.
    /// Using a priority other than `TransferPriority::Immediate` will allow the asset to be throttled
    /// via [`RenderAssetBytesPerFrame`].
    /// Specifying a size will allow the asset size to be counted towards the bytes per frame limit.
    /// If a `RenderAsset` does not implement this function, it is immediately uploaded and reports zero size.
    #[inline]
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    fn transfer_priority(
        source_asset: &Self::SourceAsset,
    ) -> (RenderAssetTransferPriority, Option<usize>) {
        // by default assets are transferred immediately, and do not count towards the frame limit.
        (RenderAssetTransferPriority::Immediate, None)
    }

    /// Prepares the [`RenderAsset::SourceAsset`] for the GPU by transforming it into a [`RenderAsset`].
    ///
    /// ECS data may be accessed via `param`.
    fn prepare_asset(
        source_asset: Self::SourceAsset,
        asset_id: AssetId<Self::SourceAsset>,
        param: &mut SystemParamItem<Self::Param>,
        previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>>;

    /// Called whenever the [`RenderAsset::SourceAsset`] has been removed.
    ///
    /// You can implement this method if you need to access ECS data (via
    /// `_param`) in order to perform cleanup tasks when the asset is removed.
    ///
    /// The default implementation does nothing.
    fn unload_asset(
        _source_asset: AssetId<Self::SourceAsset>,
        _param: &mut SystemParamItem<Self::Param>,
    ) {
    }

    /// Make a copy of the asset to be moved to the `RenderWorld` / gpu. Heavy internal data (pixels, vertex attributes)
    /// should be moved into the copy, leaving this asset with only metadata.
    /// An error may be returned to indicate that the asset has already been extracted, and should not
    /// have been modified on the CPU side (as it cannot be transferred to GPU again).
    /// The previous GPU asset is also provided, which can be used to check if the modification is valid.
    fn take_gpu_data(
        _source: &mut Self::SourceAsset,
        _previous_gpu_asset: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        Err(AssetExtractionError::NoExtractionImplementation)
    }
}

/// This plugin extracts the changed assets from the "app world" into the "render world"
/// and prepares them for the GPU. They can then be accessed from the [`RenderAssets`] resource.
///
/// Therefore it sets up the [`ExtractSchedule`] and
/// [`RenderSystems::PrepareAssets`] steps for the specified [`RenderAsset`].
///
/// The `AFTER` generic parameter can be used to specify that `A::prepare_asset` should not be run until
/// `prepare_assets::<AFTER>` has completed. This allows the `prepare_asset` function to depend on another
/// prepared [`RenderAsset`], for example `Mesh::prepare_asset` relies on `RenderAssets::<GpuImage>` for morph
/// targets, so the plugin is created as `RenderAssetPlugin::<RenderMesh, GpuImage>::default()`.
pub struct RenderAssetPlugin<A: RenderAsset, AFTER: RenderAssetDependency + 'static = ()> {
    phantom: PhantomData<fn() -> (A, AFTER)>,
}

impl<A: RenderAsset, AFTER: RenderAssetDependency + 'static> Default
    for RenderAssetPlugin<A, AFTER>
{
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<A: RenderAsset, AFTER: RenderAssetDependency + 'static> Plugin
    for RenderAssetPlugin<A, AFTER>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<CachedExtractRenderAssetSystemState<A>>();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedAssets<A>>()
                .init_resource::<RenderAssets<A>>()
                .init_resource::<PrepareNextFrameAssets<A>>()
                .add_systems(
                    ExtractSchedule,
                    extract_render_asset::<A>.in_set(AssetExtractionSystems),
                );
            AFTER::register_system::<A>(render_app);
        }
    }
}

// helper to allow specifying dependencies between render assets
pub trait RenderAssetDependency {
    fn register_system<A: RenderAsset>(render_app: &mut SubApp);
}

impl RenderAssetDependency for () {
    fn register_system<A: RenderAsset>(render_app: &mut SubApp) {
        render_app.add_systems(
            Render,
            request_bytes::<A>
                .after(reset_render_asset_bytes_per_frame)
                .before(allocate_render_asset_bytes_per_frame_priorities)
                .in_set(RenderSystems::PrepareAssets)
                .run_if(|bpf: Res<RenderAssetBytesPerFrameLimiter>| bpf.needs_requests()),
        );

        render_app.add_systems(
            Render,
            prepare_assets::<A>
                .after(allocate_render_asset_bytes_per_frame_priorities)
                .in_set(RenderSystems::PrepareAssets),
        );
    }
}

impl<AFTER: RenderAsset> RenderAssetDependency for AFTER {
    fn register_system<A: RenderAsset>(render_app: &mut SubApp) {
        render_app.add_systems(
            Render,
            request_bytes::<A>
                .after(reset_render_asset_bytes_per_frame)
                .before(allocate_render_asset_bytes_per_frame_priorities)
                .in_set(RenderSystems::PrepareAssets)
                .run_if(|bpf: Res<RenderAssetBytesPerFrameLimiter>| bpf.needs_requests()),
        );

        render_app.add_systems(
            Render,
            prepare_assets::<A>
                .after(allocate_render_asset_bytes_per_frame_priorities)
                .after(prepare_assets::<AFTER>)
                .in_set(RenderSystems::PrepareAssets),
        );
    }
}

/// Temporarily stores the extracted and removed assets of the current frame.
#[derive(Resource)]
pub struct ExtractedAssets<A: RenderAsset> {
    /// The assets extracted this frame.
    ///
    /// These are assets that were either added or modified this frame.
    pub extracted: Vec<(AssetId<A::SourceAsset>, A::SourceAsset)>,

    /// IDs of the assets that were removed this frame.
    ///
    /// These assets will not be present in [`ExtractedAssets::extracted`].
    pub removed: HashSet<AssetId<A::SourceAsset>>,

    /// IDs of the assets that were modified this frame.
    pub modified: HashSet<AssetId<A::SourceAsset>>,

    /// IDs of the assets that were added this frame.
    pub added: HashSet<AssetId<A::SourceAsset>>,
}

impl<A: RenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
            modified: Default::default(),
            added: Default::default(),
        }
    }
}

/// Stores all GPU representations ([`RenderAsset`])
/// of [`RenderAsset::SourceAsset`] as long as they exist,
/// and whether the asset is stale / queued for replacement
#[derive(Resource)]
pub struct RenderAssets<A: RenderAsset>(HashMap<AssetId<A::SourceAsset>, (A, bool)>);

impl<A: RenderAsset> Default for RenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<A: RenderAsset> RenderAssets<A> {
    pub fn get(&self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&A> {
        self.0.get(&id.into()).map(|(asset, _)| asset)
    }

    pub fn get_latest(&self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&A> {
        self.0
            .get(&id.into())
            .filter(|(_, stale)| !stale)
            .map(|(asset, _)| asset)
    }

    pub fn get_mut(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&mut A> {
        self.0.get_mut(&id.into()).map(|(asset, _)| asset)
    }

    pub fn insert(&mut self, id: impl Into<AssetId<A::SourceAsset>>, value: A) -> Option<A> {
        self.0
            .insert(id.into(), (value, false))
            .map(|(asset, _)| asset)
    }

    pub fn remove(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<A> {
        self.0.remove(&id.into()).map(|(asset, _)| asset)
    }

    pub fn set_stale(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&A> {
        self.0.get_mut(&id.into()).map(|(asset, stale)| {
            *stale = true;
            &*asset
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &A)> {
        self.0.iter().map(|(k, (asset, _))| (*k, asset))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &mut A)> {
        self.0.iter_mut().map(|(k, (asset, _))| (*k, asset))
    }
}

#[derive(Resource)]
struct CachedExtractRenderAssetSystemState<A: RenderAsset> {
    state: SystemState<(
        MessageReader<'static, 'static, AssetEvent<A::SourceAsset>>,
        ResMut<'static, Assets<A::SourceAsset>>,
        Option<Res<'static, RenderAssets<A>>>,
    )>,
}

impl<A: RenderAsset> FromWorld for CachedExtractRenderAssetSystemState<A> {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        Self {
            state: SystemState::new(world),
        }
    }
}

/// This system extracts all created or modified assets of the corresponding [`RenderAsset::SourceAsset`] type
/// into the "render world".
pub(crate) fn extract_render_asset<A: RenderAsset>(
    mut commands: Commands,
    mut main_world: ResMut<MainWorld>,
) {
    main_world.resource_scope(
        |world, mut cached_state: Mut<CachedExtractRenderAssetSystemState<A>>| {
            let (mut events, mut assets, maybe_render_assets) = cached_state.state.get_mut(world);

            let mut needs_extracting = <HashSet<_>>::default();
            let mut removed = <HashSet<_>>::default();
            let mut modified = <HashSet<_>>::default();

            for event in events.read() {
                #[expect(
                    clippy::match_same_arms,
                    reason = "LoadedWithDependencies is marked as a TODO, so it's likely this will no longer lint soon."
                )]
                match event {
                    AssetEvent::Added { id } => {
                        needs_extracting.insert(*id);
                    }
                    AssetEvent::Modified { id } => {
                        needs_extracting.insert(*id);
                        modified.insert(*id);
                    }
                    AssetEvent::Removed { .. } => {
                        // We don't care that the asset was removed from Assets<T> in the main world.
                        // An asset is only removed from RenderAssets<T> when its last handle is dropped (AssetEvent::Unused).
                    }
                    AssetEvent::Unused { id } => {
                        needs_extracting.remove(id);
                        modified.remove(id);
                        removed.insert(*id);
                    }
                    AssetEvent::LoadedWithDependencies { .. } => {
                        // TODO: handle this
                    }
                }
            }

            let mut extracted_assets = Vec::new();
            let mut added = <HashSet<_>>::default();
            for id in needs_extracting.drain() {
                if let Some(asset) = assets.get(id) {
                    let asset_usage = A::asset_usage(asset);
                    if asset_usage.contains(RenderAssetUsages::RENDER_WORLD) {
                        if asset_usage == RenderAssetUsages::RENDER_WORLD {
                            if let Some(asset) = assets.get_mut_untracked(id) {
                                let previous_asset = maybe_render_assets.as_ref().and_then(|render_assets| render_assets.get(id));
                                match A::take_gpu_data(asset, previous_asset) {
                                    Ok(gpu_data_asset) => {
                                        extracted_assets.push((id, gpu_data_asset));
                                        added.insert(id);
                                    }
                                    Err(e) => {
                                        error!("{} with RenderAssetUsages == RENDER_WORLD cannot be extracted: {e}", core::any::type_name::<A>());
                                    }
                                };
                            }
                        } else {
                            extracted_assets.push((id, asset.clone()));
                            added.insert(id);
                        }
                    }
                }
            }

            commands.insert_resource(ExtractedAssets::<A> {
                extracted: extracted_assets,
                removed,
                modified,
                added,
            });
            cached_state.state.apply(world);
        },
    );
}

// TODO: consider storing inside system?
/// All assets that should be prepared next frame.
#[derive(Resource)]
pub struct PrepareNextFrameAssets<A: RenderAsset> {
    assets: Vec<(AssetId<A::SourceAsset>, A::SourceAsset)>,
}

impl<A: RenderAsset> Default for PrepareNextFrameAssets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

/// This system iterates all assets of the corresponding [`RenderAsset::SourceAsset`] type
/// and records the bytes requested for transferring, in order to apply priorities
/// when the assets are actually prepared
pub fn request_bytes<A: RenderAsset>(
    extracted_assets: Res<ExtractedAssets<A>>,
    prepare_next_frame: Res<PrepareNextFrameAssets<A>>,
    bpf: Res<RenderAssetBytesPerFrameLimiter>,
) {
    for (id, extracted_asset) in &prepare_next_frame.assets {
        if extracted_assets.removed.contains(id) || extracted_assets.added.contains(id) {
            // skip previous frame's assets that have been removed or updated
            continue;
        }

        let (transfer_priority, maybe_bytes) = A::transfer_priority(extracted_asset);
        bpf.request_bytes(maybe_bytes, transfer_priority);
    }

    for (_, extracted_asset) in &extracted_assets.extracted {
        let (transfer_priority, maybe_bytes) = A::transfer_priority(extracted_asset);
        bpf.request_bytes(maybe_bytes, transfer_priority);
    }
}

/// This system prepares all assets of the corresponding [`RenderAsset::SourceAsset`] type
/// which where extracted this frame for the GPU.
pub fn prepare_assets<A: RenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<A>>,
    mut render_assets: ResMut<RenderAssets<A>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<A>>,
    param: StaticSystemParam<<A as RenderAsset>::Param>,
    bpf: Res<RenderAssetBytesPerFrameLimiter>,
) {
    let mut wrote_asset_count = 0;

    let mut param = param.into_inner();
    let queued_assets = core::mem::take(&mut prepare_next_frame.assets);
    for (id, extracted_asset) in queued_assets {
        if extracted_assets.removed.contains(&id) || extracted_assets.added.contains(&id) {
            // skip previous frame's assets that have been removed or updated
            continue;
        }

        let (transfer_priority, maybe_bytes) = A::transfer_priority(&extracted_asset);
        if bpf.exhausted(transfer_priority) {
            prepare_next_frame.assets.push((id, extracted_asset));
            continue;
        }

        let previous_asset = render_assets.get(id);
        match A::prepare_asset(extracted_asset, id, &mut param, previous_asset) {
            Ok(prepared_asset) => {
                render_assets.insert(id, prepared_asset);
                bpf.write_bytes(maybe_bytes, transfer_priority);
                wrote_asset_count += 1;
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
            Err(PrepareAssetError::AsBindGroupError(e)) => {
                error!(
                    "{} Bind group construction failed: {e}",
                    core::any::type_name::<A>()
                );
            }
        }
    }

    for removed in extracted_assets.removed.drain() {
        render_assets.remove(removed);
        A::unload_asset(removed, &mut param);
    }

    for (id, extracted_asset) in extracted_assets.extracted.drain(..) {
        // we do not remove previous here so that materials can continue to use
        // the old asset until it is replaced.
        // if it is necessary to have new asset immediately (e.g. if it is a resized image
        // and the shader expects the new sizes), use `RenderAssetTransferPriority::Immediate`.
        let previous_asset = render_assets.set_stale(id);

        let (transfer_priority, maybe_bytes) = A::transfer_priority(&extracted_asset);

        if bpf.exhausted(transfer_priority) {
            prepare_next_frame.assets.push((id, extracted_asset));
            continue;
        }

        match A::prepare_asset(extracted_asset, id, &mut param, previous_asset) {
            Ok(prepared_asset) => {
                render_assets.insert(id, prepared_asset);
                bpf.write_bytes(maybe_bytes, transfer_priority);
                wrote_asset_count += 1;
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
            Err(PrepareAssetError::AsBindGroupError(e)) => {
                error!(
                    "{} Bind group construction failed: {e}",
                    core::any::type_name::<A>()
                );
            }
        }
    }

    if bpf.overflowing() && !prepare_next_frame.assets.is_empty() {
        debug!(
            "{} write budget exhausted with {} assets remaining (wrote {})",
            core::any::type_name::<A>(),
            prepare_next_frame.assets.len(),
            wrote_asset_count
        );
    }
}

pub fn extract_render_asset_bytes_per_frame(
    bpf: Extract<Res<RenderAssetBytesPerFrame>>,
    mut bpf_limiter: ResMut<RenderAssetBytesPerFrameLimiter>,
) {
    bpf_limiter.bytes_per_frame = **bpf;
}

pub fn reset_render_asset_bytes_per_frame(
    mut bpf_limiter: ResMut<RenderAssetBytesPerFrameLimiter>,
) {
    bpf_limiter.reset();
}

pub fn allocate_render_asset_bytes_per_frame_priorities(
    mut bpf_limiter: ResMut<RenderAssetBytesPerFrameLimiter>,
) {
    bpf_limiter.allocate_priorities();
}

/// A resource that defines the amount of data allowed to be transferred from CPU to GPU
/// each frame, preventing choppy frames at the cost of waiting longer for GPU assets
/// to become available.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub enum RenderAssetBytesPerFrame {
    #[default]
    Unlimited,
    // apply a throttle to the total transferred bytes per frame
    MaxBytes(usize),
    // apply a throttle with prioritization using `RenderAssetTransferPriority`
    MaxBytesWithPriority(usize),
}

impl RenderAssetBytesPerFrame {
    /// `max_bytes`: the number of bytes to write per frame.
    ///
    /// This is a soft limit: only full assets are written currently, uploading stops
    /// after the first asset that exceeds the limit.
    ///
    /// To participate, assets should implement [`RenderAsset::transfer_priority`]. If the default
    /// is not overridden, the assets are assumed to be small enough to upload without restriction.
    pub fn new(max_bytes: usize) -> Self {
        Self::MaxBytes(max_bytes)
    }

    /// `max_bytes`: the number of bytes to write per frame.
    ///
    /// This is a soft limit: only full assets are written currently, uploading stops
    /// after the first asset that exceeds the limit.
    ///
    /// To participate, assets should implement [`RenderAsset::transfer_priority`]. If the default
    /// is not overridden, the assets are assumed to be small enough to upload without restriction.
    pub fn new_with_priorities(max_bytes: usize) -> Self {
        Self::MaxBytesWithPriority(max_bytes)
    }
}

#[derive(Default, Debug)]

struct RenderAssetPriorityAllocation {
    requested: AtomicUsize,
    requested_count: AtomicUsize,
    written: AtomicUsize,
    written_count: AtomicUsize,
    available: usize,
}

/// A render-world resource that facilitates limiting the data transferred from CPU to GPU
/// each frame, preventing choppy frames at the cost of waiting longer for GPU assets
/// to become available.
#[derive(Resource, Default)]
pub struct RenderAssetBytesPerFrameLimiter {
    /// Populated by [`RenderAssetBytesPerFrame`] during extraction.
    pub bytes_per_frame: RenderAssetBytesPerFrame,
    bytes_written: bevy_platform::sync::RwLock<
        BTreeMap<RenderAssetTransferPriority, RenderAssetPriorityAllocation>,
    >,
    overflowing: AtomicBool,
}

impl RenderAssetBytesPerFrameLimiter {
    /// Reset the available bytes. Called once per frame during [`RenderSystems::PrepareAssets`] by [`crate::RenderPlugin`].
    pub fn reset(&mut self) {
        match self.bytes_per_frame {
            RenderAssetBytesPerFrame::Unlimited => return,
            RenderAssetBytesPerFrame::MaxBytes(max_bytes) => {
                self.bytes_written.write().expect("can't read bpf").insert(
                    RenderAssetTransferPriority::default(),
                    RenderAssetPriorityAllocation {
                        requested: AtomicUsize::new(0),
                        requested_count: AtomicUsize::new(0),
                        written: AtomicUsize::new(0),
                        written_count: AtomicUsize::new(0),
                        available: max_bytes,
                    },
                );
            }

            RenderAssetBytesPerFrame::MaxBytesWithPriority(_) => {
                for value in self.bytes_written.read().expect("can't read bpf").values() {
                    value.requested.store(0, Ordering::Relaxed);
                    value.written.store(0, Ordering::Relaxed);
                    value.requested_count.store(0, Ordering::Relaxed);
                    value.written_count.store(0, Ordering::Relaxed);
                }
            }
        }

        let was_overflowing = self.overflowing.swap(false, Ordering::Relaxed);
        if was_overflowing {
            debug!(
                "bpf overflowed with priority buckets: {:?}",
                self.bytes_written
            );
        }
    }

    pub fn needs_requests(&self) -> bool {
        matches!(
            self.bytes_per_frame,
            RenderAssetBytesPerFrame::MaxBytesWithPriority(_)
        )
    }

    // register a number of bytes scheduled for transfer at the given priority level
    pub fn request_bytes(&self, bytes: Option<usize>, priority: RenderAssetTransferPriority) {
        let priority = match self.bytes_per_frame {
            RenderAssetBytesPerFrame::Unlimited => return,
            RenderAssetBytesPerFrame::MaxBytes(_) => RenderAssetTransferPriority::default(),
            RenderAssetBytesPerFrame::MaxBytesWithPriority(_) => priority,
        };

        if let Some(bytes_written) = self
            .bytes_written
            .read()
            .expect("can't read bpf")
            .get(&priority)
        {
            if let Some(bytes) = bytes {
                bytes_written.requested.fetch_add(bytes, Ordering::Relaxed);
            }

            bytes_written
                .requested_count
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.bytes_written.write().expect("can't write bpf").insert(
                priority,
                RenderAssetPriorityAllocation {
                    requested: AtomicUsize::new(bytes.unwrap_or_default()),
                    requested_count: AtomicUsize::new(1),
                    written: AtomicUsize::new(0),
                    written_count: AtomicUsize::new(0),
                    available: 0,
                },
            );
        }
    }

    fn allocate_priorities(&mut self) {
        let RenderAssetBytesPerFrame::MaxBytesWithPriority(mut max_bytes) = self.bytes_per_frame
        else {
            return;
        };

        for value in self
            .bytes_written
            .write()
            .expect("can't write bpf")
            .values_mut()
            // immediate, then priority(i16::max) down to priority(i16::min)
            .rev()
        {
            let requested = value.requested.load(Ordering::Relaxed);
            value.available = requested.min(max_bytes);
            max_bytes = max_bytes.saturating_sub(requested);
        }
    }

    /// Decreases the available bytes for the current frame.
    pub fn write_bytes(&self, bytes: Option<usize>, priority: RenderAssetTransferPriority) {
        let priority = match self.bytes_per_frame {
            RenderAssetBytesPerFrame::Unlimited => return,
            RenderAssetBytesPerFrame::MaxBytes(_) => RenderAssetTransferPriority::default(),
            RenderAssetBytesPerFrame::MaxBytesWithPriority(_) => priority,
        };

        if let Some(bytes_written) = self
            .bytes_written
            .read()
            .expect("can't read bpf")
            .get(&priority)
        {
            if let Some(bytes) = bytes {
                bytes_written.written.fetch_add(bytes, Ordering::Relaxed);
            }

            bytes_written.written_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Returns `true` if there are no remaining bytes available for writing this frame.
    pub fn exhausted(&self, priority: RenderAssetTransferPriority) -> bool {
        let priority = match (self.bytes_per_frame, priority) {
            (RenderAssetBytesPerFrame::Unlimited, _)
            | (_, RenderAssetTransferPriority::Immediate) => return false,
            (RenderAssetBytesPerFrame::MaxBytes(_), _) => RenderAssetTransferPriority::default(),
            (RenderAssetBytesPerFrame::MaxBytesWithPriority(_), priority) => priority,
        };

        let exhausted = self
            .bytes_written
            .read()
            .expect("can't read bpf")
            .get(&priority)
            .is_none_or(|bw| bw.written.load(Ordering::Relaxed) >= bw.available);

        if exhausted {
            self.overflowing.store(true, Ordering::Relaxed);
        }

        exhausted
    }

    pub fn overflowing(&self) -> bool {
        self.overflowing.load(Ordering::Relaxed)
    }
}
