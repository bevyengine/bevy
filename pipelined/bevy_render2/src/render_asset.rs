use crate::{RenderApp, RenderStage};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetEvent, Assets, Handle};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, RunSystem, SystemParam, SystemParamItem},
};
use bevy_utils::{HashMap, HashSet};
use std::marker::PhantomData;

pub enum PrepareAssetError<E: Send + Sync + 'static> {
    RetryNextUpdate(E),
}

pub trait RenderAsset: Asset {
    type ExtractedAsset: Send + Sync + 'static;
    type PreparedAsset: Send + Sync + 'static;
    type Param: SystemParam;
    fn extract_asset(&self) -> Self::ExtractedAsset;
    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>>;
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
        let render_app = app.sub_app(RenderApp);
        let prepare_asset_system = PrepareAssetSystem::<A>::system(&mut render_app.world);
        render_app
            .init_resource::<ExtractedAssets<A>>()
            .init_resource::<RenderAssets<A>>()
            .init_resource::<PrepareNextFrameAssets<A>>()
            .add_system_to_stage(RenderStage::Extract, extract_render_asset::<A>)
            .add_system_to_stage(RenderStage::Prepare, prepare_asset_system);
    }
}

pub struct ExtractedAssets<A: RenderAsset> {
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

pub type RenderAssetParams<R> = (
    SResMut<ExtractedAssets<R>>,
    SResMut<RenderAssets<R>>,
    SResMut<PrepareNextFrameAssets<R>>,
    <R as RenderAsset>::Param,
);

// TODO: consider storing inside system?
pub struct PrepareNextFrameAssets<A: RenderAsset> {
    assets: Vec<(Handle<A>, A::ExtractedAsset)>,
}

impl<A: RenderAsset> Default for PrepareNextFrameAssets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

pub struct PrepareAssetSystem<R: RenderAsset>(PhantomData<R>);

impl<R: RenderAsset> RunSystem for PrepareAssetSystem<R> {
    type Param = RenderAssetParams<R>;
    fn run(
        (mut extracted_assets, mut render_assets, mut prepare_next_frame, mut param): SystemParamItem<Self::Param>,
    ) {
        let mut queued_assets = std::mem::take(&mut prepare_next_frame.assets);
        for (handle, extracted_asset) in queued_assets.drain(..) {
            match R::prepare_asset(extracted_asset, &mut param) {
                Ok(prepared_asset) => {
                    render_assets.insert(handle, prepared_asset);
                }
                Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                    prepare_next_frame.assets.push((handle, extracted_asset));
                }
            }
        }

        for removed in std::mem::take(&mut extracted_assets.removed) {
            render_assets.remove(&removed);
        }

        for (handle, extracted_asset) in std::mem::take(&mut extracted_assets.extracted) {
            match R::prepare_asset(extracted_asset, &mut param) {
                Ok(prepared_asset) => {
                    render_assets.insert(handle, prepared_asset);
                }
                Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                    prepare_next_frame.assets.push((handle, extracted_asset));
                }
            }
        }
    }
}
