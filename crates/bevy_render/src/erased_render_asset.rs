use crate::render_asset::{allocate_render_asset_bytes_per_frame_priorities, reset_render_asset_bytes_per_frame};
use crate::{
    render_resource::AsBindGroupError, ExtractSchedule, MainWorld, Render, RenderApp,
    RenderSystems, Res,
};
use bevy_app::{App, Plugin, SubApp};
use bevy_asset::{Asset, AssetEvent, AssetId, Assets, UntypedAssetId};
use bevy_asset::{RenderAssetTransferPriority, RenderAssetUsages};
use bevy_ecs::{
    prelude::{Commands, IntoScheduleConfigs, MessageReader, ResMut, Resource},
    schedule::SystemSet,
    system::{StaticSystemParam, SystemParam, SystemParamItem, SystemState},
    world::{FromWorld, Mut},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_render::render_asset::RenderAssetBytesPerFrameLimiter;
use core::marker::PhantomData;
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

/// Describes how an asset gets extracted and prepared for rendering.
///
/// In the [`ExtractSchedule`] step the [`ErasedRenderAsset::SourceAsset`] is transferred
/// from the "main world" into the "render world".
///
/// After that in the [`RenderSystems::PrepareAssets`] step the extracted asset
/// is transformed into its GPU-representation of type [`ErasedRenderAsset`].
pub trait ErasedRenderAsset: Send + Sync + 'static {
    /// The representation of the asset in the "main world".
    type SourceAsset: Asset + Clone;
    /// The target representation of the asset in the "render world".
    type ErasedAsset: Send + Sync + 'static + Sized;

    /// Specifies all ECS data required by [`ErasedRenderAsset::prepare_asset`].
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
    /// 
    /// [`RenderAssetBytesPerFrame`]: crate::render_asset::RenderAssetBytesPerFrame
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

    /// Prepares the [`ErasedRenderAsset::SourceAsset`] for the GPU by transforming it into a [`ErasedRenderAsset`].
    ///
    /// ECS data may be accessed via `param`.
    fn prepare_asset(
        source_asset: Self::SourceAsset,
        asset_id: AssetId<Self::SourceAsset>,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::ErasedAsset, PrepareAssetError<Self::SourceAsset>>;

    /// Called whenever the [`ErasedRenderAsset::SourceAsset`] has been removed.
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
}

/// This plugin extracts the changed assets from the "app world" into the "render world"
/// and prepares them for the GPU. They can then be accessed from the [`ErasedRenderAssets`] resource.
///
/// Therefore it sets up the [`ExtractSchedule`] and
/// [`RenderSystems::PrepareAssets`] steps for the specified [`ErasedRenderAsset`].
///
/// The `AFTER` generic parameter can be used to specify that `A::prepare_asset` should not be run until
/// `prepare_assets::<AFTER>` has completed. This allows the `prepare_asset` function to depend on another
/// prepared [`ErasedRenderAsset`], for example `Mesh::prepare_asset` relies on `ErasedRenderAssets::<GpuImage>` for morph
/// targets, so the plugin is created as `ErasedRenderAssetPlugin::<RenderMesh, GpuImage>::default()`.
pub struct ErasedRenderAssetPlugin<
    A: ErasedRenderAsset,
    AFTER: ErasedRenderAssetDependency + 'static = (),
> {
    phantom: PhantomData<fn() -> (A, AFTER)>,
}

impl<A: ErasedRenderAsset, AFTER: ErasedRenderAssetDependency + 'static> Default
    for ErasedRenderAssetPlugin<A, AFTER>
{
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<A: ErasedRenderAsset, AFTER: ErasedRenderAssetDependency + 'static> Plugin
    for ErasedRenderAssetPlugin<A, AFTER>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<CachedExtractErasedRenderAssetSystemState<A>>();
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedAssets<A>>()
                .init_resource::<ErasedRenderAssets<A::ErasedAsset>>()
                .init_resource::<PrepareNextFrameAssets<A>>()
                .add_systems(
                    ExtractSchedule,
                    extract_erased_render_asset::<A>.in_set(AssetExtractionSystems),
                );
            AFTER::register_system::<A>(render_app);
        }
    }
}

// helper to allow specifying dependencies between render assets
pub trait ErasedRenderAssetDependency {
    fn register_system<A: ErasedRenderAsset>(render_app: &mut SubApp);
}

impl ErasedRenderAssetDependency for () {
    fn register_system<A: ErasedRenderAsset>(render_app: &mut SubApp) {
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
            prepare_erased_assets::<A>
                .after(allocate_render_asset_bytes_per_frame_priorities)
                .in_set(RenderSystems::PrepareAssets),
        );
    }
}

impl<AFTER: ErasedRenderAsset> ErasedRenderAssetDependency for AFTER {
    fn register_system<A: ErasedRenderAsset>(render_app: &mut SubApp) {
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
            prepare_erased_assets::<A>
                .after(allocate_render_asset_bytes_per_frame_priorities)
                .after(prepare_erased_assets::<AFTER>)
                .in_set(RenderSystems::PrepareAssets),
        );
    }
}
/// Temporarily stores the extracted and removed assets of the current frame.
#[derive(Resource)]
pub struct ExtractedAssets<A: ErasedRenderAsset> {
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

impl<A: ErasedRenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
            modified: Default::default(),
            added: Default::default(),
        }
    }
}

/// Stores all GPU representations ([`ErasedRenderAsset`])
/// of [`ErasedRenderAsset::SourceAsset`] as long as they exist.
#[derive(Resource)]
pub struct ErasedRenderAssets<ERA>(HashMap<UntypedAssetId, (ERA, bool)>);

impl<ERA> Default for ErasedRenderAssets<ERA> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<ERA> ErasedRenderAssets<ERA> {
    pub fn get(&self, id: impl Into<UntypedAssetId>) -> Option<&ERA> {
        self.0.get(&id.into()).map(|(asset, _)| asset)
    }

    pub fn get_latest(&self, id: impl Into<UntypedAssetId>) -> Option<&ERA> {
        self.0
            .get(&id.into())
            .filter(|(_, stale)| !stale)
            .map(|(asset, _)| asset)
    }

    pub fn get_mut(&mut self, id: impl Into<UntypedAssetId>) -> Option<&mut ERA> {
        self.0.get_mut(&id.into()).map(|(asset, _)| asset)
    }

    pub fn insert(&mut self, id: impl Into<UntypedAssetId>, value: ERA) -> Option<ERA> {
        self.0
            .insert(id.into(), (value, false))
            .map(|(asset, _)| asset)
    }

    pub fn remove(&mut self, id: impl Into<UntypedAssetId>) -> Option<ERA> {
        self.0.remove(&id.into()).map(|(asset, _)| asset)
    }

    pub fn set_stale(&mut self, id: impl Into<UntypedAssetId>) -> Option<&ERA> {
        self.0.get_mut(&id.into()).map(|(asset, stale)| {
            *stale = true;
            &*asset
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (UntypedAssetId, &ERA)> {
        self.0.iter().map(|(k, (asset, _))| (*k, asset))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (UntypedAssetId, &mut ERA)> {
        self.0.iter_mut().map(|(k, (asset, _))| (*k, asset))
    }
}

#[derive(Resource)]
struct CachedExtractErasedRenderAssetSystemState<A: ErasedRenderAsset> {
    state: SystemState<(
        MessageReader<'static, 'static, AssetEvent<A::SourceAsset>>,
        ResMut<'static, Assets<A::SourceAsset>>,
    )>,
}

impl<A: ErasedRenderAsset> FromWorld for CachedExtractErasedRenderAssetSystemState<A> {
    fn from_world(world: &mut bevy_ecs::world::World) -> Self {
        Self {
            state: SystemState::new(world),
        }
    }
}

/// This system extracts all created or modified assets of the corresponding [`ErasedRenderAsset::SourceAsset`] type
/// into the "render world".
pub(crate) fn extract_erased_render_asset<A: ErasedRenderAsset>(
    mut commands: Commands,
    mut main_world: ResMut<MainWorld>,
) {
    main_world.resource_scope(
        |world, mut cached_state: Mut<CachedExtractErasedRenderAssetSystemState<A>>| {
            let (mut events, mut assets) = cached_state.state.get_mut(world);

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
                        // An asset is only removed from ErasedRenderAssets<T> when its last handle is dropped (AssetEvent::Unused).
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
                            if let Some(asset) = assets.remove(id) {
                                extracted_assets.push((id, asset));
                                added.insert(id);
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
pub struct PrepareNextFrameAssets<A: ErasedRenderAsset> {
    assets: Vec<(AssetId<A::SourceAsset>, A::SourceAsset)>,
}

impl<A: ErasedRenderAsset> Default for PrepareNextFrameAssets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

/// This system iterates all assets of the corresponding [`ErasedRenderAsset::SourceAsset`] type
/// and records the bytes requested for transferring, in order to apply priorities
/// when the assets are actually prepared
pub fn request_bytes<A: ErasedRenderAsset>(
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

/// This system prepares all assets of the corresponding [`ErasedRenderAsset::SourceAsset`] type
/// which where extracted this frame for the GPU.
pub fn prepare_erased_assets<A: ErasedRenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<A>>,
    mut render_assets: ResMut<ErasedRenderAssets<A::ErasedAsset>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<A>>,
    param: StaticSystemParam<<A as ErasedRenderAsset>::Param>,
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

        match A::prepare_asset(extracted_asset, id, &mut param) {
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
        let _previous_asset = render_assets.set_stale(id);

        let (transfer_priority, maybe_bytes) = A::transfer_priority(&extracted_asset);

        if bpf.exhausted(transfer_priority) {
            prepare_next_frame.assets.push((id, extracted_asset));
            continue;
        }

        match A::prepare_asset(extracted_asset, id, &mut param) {
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
