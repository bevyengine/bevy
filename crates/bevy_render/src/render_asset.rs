use crate::{ExtractSchedule, MainWorld, Render, RenderApp, RenderSet};
use bevy_app::{App, Plugin, SubApp};
use bevy_asset::{Asset, AssetEvent, AssetId, Assets};
use bevy_ecs::{
    prelude::{Commands, EventReader, IntoSystemConfigs, ResMut, Resource},
    schedule::SystemConfigs,
    system::{StaticSystemParam, SystemParam, SystemParamItem, SystemState},
    world::{FromWorld, Mut},
};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render_macros::ExtractResource;
use bevy_utils::{tracing::debug, HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrepareAssetError<E: Send + Sync + 'static> {
    #[error("Failed to prepare asset")]
    RetryNextUpdate(E),
}

/// Describes how an asset gets extracted and prepared for rendering.
///
/// In the [`ExtractSchedule`] step the [`RenderAsset::SourceAsset`] is transferred
/// from the "main world" into the "render world".
///
/// After that in the [`RenderSet::PrepareAssets`] step the extracted asset
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

    /// Size of the data the asset will upload to the gpu. Specifying a return value
    /// will allow the asset to be throttled via [`RenderAssetBytesPerFrame`].
    #[inline]
    #[allow(unused_variables)]
    fn byte_len(source_asset: &Self::SourceAsset) -> Option<usize> {
        None
    }

    /// Prepares the [`RenderAsset::SourceAsset`] for the GPU by transforming it into a [`RenderAsset`].
    ///
    /// ECS data may be accessed via `param`.
    fn prepare_asset(
        source_asset: Self::SourceAsset,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>>;
}

bitflags::bitflags! {
    /// Defines where the asset will be used.
    ///
    /// If an asset is set to the `RENDER_WORLD` but not the `MAIN_WORLD`, the asset will be
    /// unloaded from the asset server once it's been extracted and prepared in the render world.
    ///
    /// Unloading the asset saves on memory, as for most cases it is no longer necessary to keep
    /// it in RAM once it's been uploaded to the GPU's VRAM. However, this means you can no longer
    /// access the asset from the CPU (via the `Assets<T>` resource) once unloaded (without re-loading it).
    ///
    /// If you never need access to the asset from the CPU past the first frame it's loaded on,
    /// or only need very infrequent access, then set this to `RENDER_WORLD`. Otherwise, set this to
    /// `RENDER_WORLD | MAIN_WORLD`.
    ///
    /// If you have an asset that doesn't actually need to end up in the render world, like an Image
    /// that will be decoded into another Image asset, use `MAIN_WORLD` only.
    ///
    /// ## Platform-specific
    ///
    /// On Wasm, it is not possible for now to free reserved memory. To control memory usage, load assets
    /// in sequence and unload one before loading the next. See this
    /// [discussion about memory management](https://github.com/WebAssembly/design/issues/1397) for more
    /// details.
    #[repr(transparent)]
    #[derive(Serialize, Deserialize, Hash, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
    #[reflect_value(Serialize, Deserialize, Hash, PartialEq, Debug)]
    pub struct RenderAssetUsages: u8 {
        const MAIN_WORLD = 1 << 0;
        const RENDER_WORLD = 1 << 1;
    }
}

impl Default for RenderAssetUsages {
    /// Returns the default render asset usage flags:
    /// `RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD`
    ///
    /// This default configuration ensures the asset persists in the main world, even after being prepared for rendering.
    ///
    /// If your asset does not change, consider using `RenderAssetUsages::RENDER_WORLD` exclusively. This will cause
    /// the asset to be unloaded from the main world once it has been prepared for rendering. If the asset does not need
    /// to reach the render world at all, use `RenderAssetUsages::MAIN_WORLD` exclusively.
    fn default() -> Self {
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    }
}

/// This plugin extracts the changed assets from the "app world" into the "render world"
/// and prepares them for the GPU. They can then be accessed from the [`RenderAssets`] resource.
///
/// Therefore it sets up the [`ExtractSchedule`] and
/// [`RenderSet::PrepareAssets`] steps for the specified [`RenderAsset`].
///
/// The `AFTER` generic parameter can be used to specify that `A::prepare_asset` should not be run until
/// `prepare_assets::<AFTER>` has completed. This allows the `prepare_asset` function to depend on another
/// prepared [`RenderAsset`], for example `Mesh::prepare_asset` relies on `RenderAssets::<GpuImage>` for morph
/// targets, so the plugin is created as `RenderAssetPlugin::<GpuMesh, GpuImage>::default()`.
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
                .add_systems(ExtractSchedule, extract_render_asset::<A>);
            AFTER::register_system(
                render_app,
                prepare_assets::<A>.in_set(RenderSet::PrepareAssets),
            );
        }
    }
}

// helper to allow specifying dependencies between render assets
pub trait RenderAssetDependency {
    fn register_system(render_app: &mut SubApp, system: SystemConfigs);
}

impl RenderAssetDependency for () {
    fn register_system(render_app: &mut SubApp, system: SystemConfigs) {
        render_app.add_systems(Render, system);
    }
}

impl<A: RenderAsset> RenderAssetDependency for A {
    fn register_system(render_app: &mut SubApp, system: SystemConfigs) {
        render_app.add_systems(Render, system.after(prepare_assets::<A>));
    }
}

/// Temporarily stores the extracted and removed assets of the current frame.
#[derive(Resource)]
pub struct ExtractedAssets<A: RenderAsset> {
    extracted: Vec<(AssetId<A::SourceAsset>, A::SourceAsset)>,
    removed: HashSet<AssetId<A::SourceAsset>>,
    added: HashSet<AssetId<A::SourceAsset>>,
}

impl<A: RenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
            added: Default::default(),
        }
    }
}

/// Stores all GPU representations ([`RenderAsset`])
/// of [`RenderAsset::SourceAsset`] as long as they exist.
#[derive(Resource)]
pub struct RenderAssets<A: RenderAsset>(HashMap<AssetId<A::SourceAsset>, A>);

impl<A: RenderAsset> Default for RenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<A: RenderAsset> RenderAssets<A> {
    pub fn get(&self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&A> {
        self.0.get(&id.into())
    }

    pub fn get_mut(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&mut A> {
        self.0.get_mut(&id.into())
    }

    pub fn insert(&mut self, id: impl Into<AssetId<A::SourceAsset>>, value: A) -> Option<A> {
        self.0.insert(id.into(), value)
    }

    pub fn remove(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<A> {
        self.0.remove(&id.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &A)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &mut A)> {
        self.0.iter_mut().map(|(k, v)| (*k, v))
    }
}

#[derive(Resource)]
struct CachedExtractRenderAssetSystemState<A: RenderAsset> {
    state: SystemState<(
        EventReader<'static, 'static, AssetEvent<A::SourceAsset>>,
        ResMut<'static, Assets<A::SourceAsset>>,
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
fn extract_render_asset<A: RenderAsset>(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    main_world.resource_scope(
        |world, mut cached_state: Mut<CachedExtractRenderAssetSystemState<A>>| {
            let (mut events, mut assets) = cached_state.state.get_mut(world);

            let mut changed_assets = HashSet::default();
            let mut removed = HashSet::default();

            for event in events.read() {
                #[allow(clippy::match_same_arms)]
                match event {
                    AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                        changed_assets.insert(*id);
                    }
                    AssetEvent::Removed { .. } => {}
                    AssetEvent::Unused { id } => {
                        changed_assets.remove(id);
                        removed.insert(*id);
                    }
                    AssetEvent::LoadedWithDependencies { .. } => {
                        // TODO: handle this
                    }
                }
            }

            let mut extracted_assets = Vec::new();
            let mut added = HashSet::new();
            for id in changed_assets.drain() {
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

/// This system prepares all assets of the corresponding [`RenderAsset::SourceAsset`] type
/// which where extracted this frame for the GPU.
pub fn prepare_assets<A: RenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<A>>,
    mut render_assets: ResMut<RenderAssets<A>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<A>>,
    param: StaticSystemParam<<A as RenderAsset>::Param>,
    mut bpf: ResMut<RenderAssetBytesPerFrame>,
) {
    let mut wrote_asset_count = 0;

    let mut param = param.into_inner();
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (id, extracted_asset) in queued_assets {
        if extracted_assets.removed.contains(&id) || extracted_assets.added.contains(&id) {
            // skip previous frame's assets that have been removed or updated
            continue;
        }

        let write_bytes = if let Some(size) = A::byte_len(&extracted_asset) {
            // we could check if available bytes > byte_len here, but we want to make some
            // forward progress even if the asset is larger than the max bytes per frame.
            // this way we always write at least one (sized) asset per frame.
            // in future we could also consider partial asset uploads.
            if bpf.exhausted() {
                prepare_next_frame.assets.push((id, extracted_asset));
                continue;
            }
            size
        } else {
            0
        };

        match A::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(id, prepared_asset);
                bpf.write_bytes(write_bytes);
                wrote_asset_count += 1;
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
        }
    }

    for removed in extracted_assets.removed.drain() {
        render_assets.remove(removed);
    }

    for (id, extracted_asset) in extracted_assets.extracted.drain(..) {
        // we remove previous here to ensure that if we are updating the asset then
        // any users will not see the old asset after a new asset is extracted,
        // even if the new asset is not yet ready or we are out of bytes to write.
        render_assets.remove(id);

        let write_bytes = if let Some(size) = A::byte_len(&extracted_asset) {
            if bpf.exhausted() {
                prepare_next_frame.assets.push((id, extracted_asset));
                continue;
            }
            size
        } else {
            0
        };

        match A::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(id, prepared_asset);
                bpf.write_bytes(write_bytes);
                wrote_asset_count += 1;
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
        }
    }

    if bpf.exhausted() && !prepare_next_frame.assets.is_empty() {
        debug!(
            "{} write budget exhausted with {} assets remaining (wrote {})",
            std::any::type_name::<A>(),
            prepare_next_frame.assets.len(),
            wrote_asset_count
        );
    }
}

/// A resource that attempts to limit the amount of data transferred from cpu to gpu
/// each frame, preventing choppy frames at the cost of waiting longer for gpu assets
/// to become available
#[derive(Resource, Default, Debug, Clone, Copy, ExtractResource)]
pub struct RenderAssetBytesPerFrame {
    pub max_bytes: Option<usize>,
    pub available: usize,
}

impl RenderAssetBytesPerFrame {
    /// `max_bytes`: the number of bytes to write per frame.
    /// this is a soft limit: only full assets are written currently, uploading stops
    /// after the first asset that exceeds the limit.
    /// To participate, assets should implement [`RenderAsset::byte_len`]. If the default
    /// is not overridden, the assets are assumed to be small enough to upload without restriction.
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes: Some(max_bytes),
            available: 0,
        }
    }

    /// Reset the available bytes. Called once per frame by the [`crate::RenderPlugin`].
    pub fn reset(&mut self) {
        self.available = self.max_bytes.unwrap_or(usize::MAX);
    }

    /// check how many bytes are available since the last reset
    pub fn available_bytes(&self, required_bytes: usize) -> usize {
        if self.max_bytes.is_none() {
            return required_bytes;
        }

        required_bytes.min(self.available)
    }

    /// decrease the available bytes for the current frame
    fn write_bytes(&mut self, bytes: usize) {
        if self.max_bytes.is_none() {
            return;
        }

        let write_bytes = bytes.min(self.available);
        self.available -= write_bytes;
    }

    // check if any bytes remain available for writing this frame
    fn exhausted(&self) -> bool {
        self.max_bytes.is_some() && self.available == 0
    }
}
