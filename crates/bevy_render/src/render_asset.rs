use crate::{ExtractSchedule, MainWorld, Render, RenderApp, RenderSet};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetEvent, AssetId, Assets};
use bevy_ecs::{
    prelude::{Commands, EventReader, IntoSystemConfigs, ResMut, Resource},
    schedule::SystemConfigs,
    system::{StaticSystemParam, SystemParam, SystemParamItem, SystemState},
};
use bevy_reflect::Reflect;
use bevy_utils::{thiserror::Error, HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Debug, Error)]
pub enum PrepareAssetError<E: Send + Sync + 'static> {
    #[error("Failed to prepare asset")]
    RetryNextUpdate(E),
}

/// Describes how an asset gets extracted and prepared for rendering.
///
/// In the [`ExtractSchedule`] step the asset is transferred
/// from the "main world" into the "render world".
///
/// After that in the [`RenderSet::PrepareAssets`] step the extracted asset
/// is transformed into its GPU-representation of type [`RenderAsset::PreparedAsset`].
pub trait RenderAsset: Asset + Clone {
    /// The GPU-representation of the asset.
    type PreparedAsset: Send + Sync + 'static;

    /// Specifies all ECS data required by [`RenderAsset::prepare_asset`].
    ///
    /// For convenience use the [`lifetimeless`](bevy_ecs::system::lifetimeless) [`SystemParam`].
    type Param: SystemParam;

    /// Whether or not to unload the asset after extracting it to the render world.
    fn persistence_policy(&self) -> RenderAssetPersistencePolicy;

    /// Prepares the asset for the GPU by transforming it into a [`RenderAsset::PreparedAsset`].
    ///
    /// ECS data may be accessed via `param`.
    fn prepare_asset(
        self,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>>;
}

/// Whether or not to unload the [`RenderAsset`] after extracting it to the render world.
///
/// Unloading the asset saves on memory, as for most cases it is no longer necessary to keep
/// it in RAM once it's been uploaded to the GPU's VRAM. However, this means you can no longer
/// access the asset from the CPU (via the `Assets<T>` resource) once unloaded (without re-loading it).
///
/// If you never need access to the asset from the CPU past the first frame it's loaded on,
/// or only need very infrequent access, then set this to Unload. Otherwise, set this to Keep.
#[derive(Reflect, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum RenderAssetPersistencePolicy {
    Unload,
    #[default]
    Keep,
}

/// This plugin extracts the changed assets from the "app world" into the "render world"
/// and prepares them for the GPU. They can then be accessed from the [`RenderAssets`] resource.
///
/// Therefore it sets up the [`ExtractSchedule`] and
/// [`RenderSet::PrepareAssets`] steps for the specified [`RenderAsset`].
///
/// The `AFTER` generic parameter can be used to specify that `A::prepare_asset` should not be run until
/// `prepare_assets::<AFTER>` has completed. This allows the `prepare_asset` function to depend on another
/// prepared [`RenderAsset`], for example `Mesh::prepare_asset` relies on `RenderAssets::<Image>` for morph
/// targets, so the plugin is created as `RenderAssetPlugin::<Mesh, Image>::default()`.
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
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
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
    fn register_system(render_app: &mut App, system: SystemConfigs);
}

impl RenderAssetDependency for () {
    fn register_system(render_app: &mut App, system: SystemConfigs) {
        render_app.add_systems(Render, system);
    }
}

impl<A: RenderAsset> RenderAssetDependency for A {
    fn register_system(render_app: &mut App, system: SystemConfigs) {
        render_app.add_systems(Render, system.after(prepare_assets::<A>));
    }
}

/// Temporarily stores the extracted and removed assets of the current frame.
#[derive(Resource)]
pub struct ExtractedAssets<A: RenderAsset> {
    extracted: Vec<(AssetId<A>, A)>,
    removed: Vec<AssetId<A>>,
}

impl<A: RenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

/// Stores all GPU representations ([`RenderAsset::PreparedAssets`](RenderAsset::PreparedAsset))
/// of [`RenderAssets`](RenderAsset) as long as they exist.
#[derive(Resource)]
pub struct RenderAssets<A: RenderAsset>(HashMap<AssetId<A>, A::PreparedAsset>);

impl<A: RenderAsset> Default for RenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<A: RenderAsset> RenderAssets<A> {
    pub fn get(&self, id: impl Into<AssetId<A>>) -> Option<&A::PreparedAsset> {
        self.0.get(&id.into())
    }

    pub fn get_mut(&mut self, id: impl Into<AssetId<A>>) -> Option<&mut A::PreparedAsset> {
        self.0.get_mut(&id.into())
    }

    pub fn insert(
        &mut self,
        id: impl Into<AssetId<A>>,
        value: A::PreparedAsset,
    ) -> Option<A::PreparedAsset> {
        self.0.insert(id.into(), value)
    }

    pub fn remove(&mut self, id: impl Into<AssetId<A>>) -> Option<A::PreparedAsset> {
        self.0.remove(&id.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId<A>, &A::PreparedAsset)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetId<A>, &mut A::PreparedAsset)> {
        self.0.iter_mut().map(|(k, v)| (*k, v))
    }
}

/// This system extracts all created or modified assets of the corresponding [`RenderAsset`] type
/// into the "render world".
fn extract_render_asset<A: RenderAsset>(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut system_state: SystemState<(EventReader<AssetEvent<A>>, ResMut<Assets<A>>)> =
        SystemState::new(&mut main_world);
    let (mut events, mut assets) = system_state.get_mut(&mut main_world);

    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.read() {
        #[allow(clippy::match_same_arms)]
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
            }
            AssetEvent::Removed { .. } => {}
            AssetEvent::Unused { id } => {
                changed_assets.remove(id);
                removed.push(*id);
            }
            AssetEvent::LoadedWithDependencies { .. } => {
                // TODO: handle this
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for id in changed_assets.drain() {
        if let Some(asset) = assets.get(id) {
            if asset.persistence_policy() == RenderAssetPersistencePolicy::Unload {
                if let Some(asset) = assets.remove(id) {
                    extracted_assets.push((id, asset));
                }
            } else {
                extracted_assets.push((id, asset.clone()));
            }
        }
    }

    commands.insert_resource(ExtractedAssets {
        extracted: extracted_assets,
        removed,
    });
}

// TODO: consider storing inside system?
/// All assets that should be prepared next frame.
#[derive(Resource)]
pub struct PrepareNextFrameAssets<A: RenderAsset> {
    assets: Vec<(AssetId<A>, A)>,
}

impl<A: RenderAsset> Default for PrepareNextFrameAssets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

/// This system prepares all assets of the corresponding [`RenderAsset`] type
/// which where extracted this frame for the GPU.
pub fn prepare_assets<A: RenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<A>>,
    mut render_assets: ResMut<RenderAssets<A>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<A>>,
    param: StaticSystemParam<<A as RenderAsset>::Param>,
) {
    let mut param = param.into_inner();
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (id, extracted_asset) in queued_assets {
        match extracted_asset.prepare_asset(&mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
        }
    }

    for removed in extracted_assets.removed.drain(..) {
        render_assets.remove(removed);
    }

    for (id, extracted_asset) in extracted_assets.extracted.drain(..) {
        match extracted_asset.prepare_asset(&mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
        }
    }
}
