use bevy_asset::AssetId;
use bevy_core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_render::{
    render_asset::{PrepareAssetError, RenderAsset},
    render_phase::DrawFunctions,
    render_resource::{AsBindGroupError, BindGroup, BindingResources},
    renderer::RenderDevice,
};

use crate::{
    material::{AlphaMode2d, Material2d},
    mesh_pipeline::{key::Mesh2dPipelineKey, render::Material2dBindGroupId},
};

use super::{
    commands::DrawMaterial2d, pipeline::Material2dPipeline, properties::Material2dProperties,
};

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
