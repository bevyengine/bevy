use std::marker::PhantomData;

use crate::{
    renderer::{RenderDevice, RenderQueue},
    RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetEvent, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_utils::{HashMap, HashSet};

pub trait RenderAsset: Asset {
    type ExtractedAsset: Send + Sync + 'static;
    type PreparedAsset: Send + Sync + 'static;
    fn extract_asset(&self) -> Self::ExtractedAsset;
    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Self::PreparedAsset;
}

/// Extracts assets into gpu-usable data
pub struct RenderAssetPlugin<A: RenderAsset>(PhantomData<fn() -> A>);

impl<A: RenderAsset> Default for RenderAssetPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: RenderAsset> Plugin for RenderAssetPlugin<A> {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(0);
        render_app
            .init_resource::<ExtractedAssets<A>>()
            .init_resource::<RenderAssets<A>>()
            .add_system_to_stage(RenderStage::Extract, extract_render_asset::<A>.system())
            .add_system_to_stage(RenderStage::Prepare, prepare_render_asset::<A>.system());
    }
}

struct ExtractedAssets<A: RenderAsset> {
    extracted: Vec<(Handle<A>, A::ExtractedAsset)>,
    removed: Vec<Handle<A>>,
}

impl<A: RenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

pub type RenderAssets<A> = HashMap<Handle<A>, <A as RenderAsset>::PreparedAsset>;

fn extract_render_asset<A: RenderAsset>(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<A>>,
    assets: Res<Assets<A>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                changed_assets.insert(handle);
            }
            AssetEvent::Modified { handle } => {
                changed_assets.insert(handle);
            }
            AssetEvent::Removed { handle } => {
                if !changed_assets.remove(handle) {
                    removed.push(handle.clone_weak());
                }
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(asset) = assets.get(handle) {
            extracted_assets.push((handle.clone_weak(), asset.extract_asset()));
        }
    }

    commands.insert_resource(ExtractedAssets {
        extracted: extracted_assets,
        removed,
    })
}

fn prepare_render_asset<R: RenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<R>>,
    mut render_assets: ResMut<RenderAssets<R>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for removed in extracted_assets.removed.iter() {
        render_assets.remove(removed);
    }

    for (handle, extracted_asset) in extracted_assets.extracted.drain(..) {
        let prepared_asset = R::prepare_asset(extracted_asset, &render_device, &render_queue);
        render_assets.insert(handle, prepared_asset);
    }
}
