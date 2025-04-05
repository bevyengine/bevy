use core::marker::PhantomData;

use bevy_asset::AssetId;
use bevy_core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Tick,
    entity::Entity,
    resource::Resource,
    system::{lifetimeless::SRes, SystemParamItem},
};
use bevy_platform_support::collections::HashMap;
use bevy_render::{
    render_asset::{PrepareAssetError, RenderAsset},
    render_phase::{DrawFunctionId, DrawFunctions},
    render_resource::{AsBindGroupError, BindGroup, BindingResources, CachedRenderPipelineId},
    renderer::RenderDevice,
    sync_world::MainEntityHashMap,
};

use crate::mesh_pipeline::{key::Mesh2dPipelineKey, render::Material2dBindGroupId};

use super::{commands::DrawMaterial2d, pipeline::Material2dPipeline, AlphaMode2d, Material2d};

#[derive(Resource, Deref, DerefMut)]
pub struct RenderMaterial2dInstances<M: Material2d>(MainEntityHashMap<AssetId<M>>);

impl<M: Material2d> Default for RenderMaterial2dInstances<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Data prepared for a [`Material2d`] instance.
pub struct PreparedMaterial2d<T: Material2d> {
    #[expect(dead_code, reason = "`dead_code` under investigation")]
    pub bindings: BindingResources,
    pub bind_group: BindGroup,
    pub key: T::Data,
    pub properties: Material2dProperties,
}

impl<T: Material2d> PreparedMaterial2d<T> {
    pub fn get_bind_group_id(&self) -> Material2dBindGroupId {
        Material2dBindGroupId(Some(self.bind_group.id()))
    }
}

impl<M: Material2d> RenderAsset for PreparedMaterial2d<M> {
    type SourceAsset = M;

    type Param = (
        SRes<RenderDevice>,
        SRes<Material2dPipeline<M>>,
        SRes<DrawFunctions<Opaque2d>>,
        SRes<DrawFunctions<AlphaMask2d>>,
        SRes<DrawFunctions<Transparent2d>>,
        M::Param,
    );

    fn prepare_asset(
        material: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (
            render_device,
            pipeline,
            opaque_draw_functions,
            alpha_mask_draw_functions,
            transparent_draw_functions,
            material_param,
        ): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        match material.as_bind_group(&pipeline.material2d_layout, render_device, material_param) {
            Ok(prepared) => {
                let mut mesh_pipeline_key_bits = Mesh2dPipelineKey::empty();
                mesh_pipeline_key_bits.insert(alpha_mode_pipeline_key(material.alpha_mode()));

                let draw_function_id = match material.alpha_mode() {
                    AlphaMode2d::Opaque => opaque_draw_functions.read().id::<DrawMaterial2d<M>>(),
                    AlphaMode2d::Mask(_) => {
                        alpha_mask_draw_functions.read().id::<DrawMaterial2d<M>>()
                    }
                    AlphaMode2d::Blend => {
                        transparent_draw_functions.read().id::<DrawMaterial2d<M>>()
                    }
                };

                Ok(PreparedMaterial2d {
                    bindings: prepared.bindings,
                    bind_group: prepared.bind_group,
                    key: prepared.data,
                    properties: Material2dProperties {
                        depth_bias: material.depth_bias(),
                        alpha_mode: material.alpha_mode(),
                        mesh_pipeline_key_bits,
                        draw_function_id,
                    },
                })
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                Err(PrepareAssetError::RetryNextUpdate(material))
            }
            Err(other) => Err(PrepareAssetError::AsBindGroupError(other)),
        }
    }
}

const fn alpha_mode_pipeline_key(alpha_mode: AlphaMode2d) -> Mesh2dPipelineKey {
    match alpha_mode {
        AlphaMode2d::Blend => Mesh2dPipelineKey::BLEND_ALPHA,
        AlphaMode2d::Mask(_) => Mesh2dPipelineKey::MAY_DISCARD,
        _ => Mesh2dPipelineKey::NONE,
    }
}

/// Common [`Material2d`] properties, calculated for a specific material instance.
pub struct Material2dProperties {
    /// The [`AlphaMode2d`] of this material.
    pub alpha_mode: AlphaMode2d,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may
    pub depth_bias: f32,
    /// The bits in the [`Mesh2dPipelineKey`] for this material.
    ///
    /// [`Mesh2dPipelineKey`] are precalculated so that we can just "or" them together.
    pub mesh_pipeline_key_bits: Mesh2dPipelineKey,
    pub draw_function_id: DrawFunctionId,
}

#[derive(Clone, Resource, Deref, DerefMut, Debug)]
pub struct EntitiesNeedingSpecialization<M> {
    #[deref]
    pub entities: Vec<Entity>,
    _marker: PhantomData<M>,
}

impl<M> Default for EntitiesNeedingSpecialization<M> {
    fn default() -> Self {
        Self {
            entities: Default::default(),
            _marker: Default::default(),
        }
    }
}

#[derive(Clone, Resource, Deref, DerefMut, Debug)]
pub struct EntitySpecializationTicks<M> {
    #[deref]
    pub entities: MainEntityHashMap<Tick>,
    _marker: PhantomData<M>,
}

impl<M> Default for EntitySpecializationTicks<M> {
    fn default() -> Self {
        Self {
            entities: MainEntityHashMap::default(),
            _marker: Default::default(),
        }
    }
}

/// Stores the [`SpecializedMaterial2dViewPipelineCache`] for each view.
#[derive(Resource, Deref, DerefMut)]
pub struct SpecializedMaterial2dPipelineCache<M> {
    // view_entity -> view pipeline cache
    #[deref]
    map: MainEntityHashMap<SpecializedMaterial2dViewPipelineCache<M>>,
    marker: PhantomData<M>,
}

/// Stores the cached render pipeline ID for each entity in a single view, as
/// well as the last time it was changed.
#[derive(Deref, DerefMut)]
pub struct SpecializedMaterial2dViewPipelineCache<M> {
    // material entity -> (tick, pipeline_id)
    #[deref]
    map: MainEntityHashMap<(Tick, CachedRenderPipelineId)>,
    marker: PhantomData<M>,
}

impl<M> Default for SpecializedMaterial2dPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            marker: PhantomData,
        }
    }
}

impl<M> Default for SpecializedMaterial2dViewPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            marker: PhantomData,
        }
    }
}
