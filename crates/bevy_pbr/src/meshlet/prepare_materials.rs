use super::MeshletGpuScene;
use crate::*;
use bevy_core_pipeline::{
    experimental::taa::TemporalAntiAliasSettings,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_render::{
    camera::Projection,
    render_asset::RenderAssets,
    render_resource::{BindGroup, CachedRenderPipelineId, PipelineCache, RenderPipelineDescriptor},
    view::ExtractedView,
};
use bevy_utils::HashMap;
use std::hash::Hash;

#[derive(Component)]
pub struct MeshletViewMaterials {
    pub opaque_pass: Vec<(u32, CachedRenderPipelineId, BindGroup)>,
    pub prepass: Vec<(u32, CachedRenderPipelineId, BindGroup)>,
}

/// TODO: How to differentiate between main/prepass? Also, check not missing any keys/shaderdefs
#[allow(clippy::too_many_arguments)]
pub fn prepare_material_meshlet_meshes<M: Material>(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    mut cache: Local<HashMap<MeshPipelineKey, CachedRenderPipelineId>>,
    pipeline_cache: Res<PipelineCache>,
    render_materials: Res<RenderMaterials<M>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    images: Res<RenderAssets<Image>>,
    views: Query<(
        Entity,
        &ExtractedView,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&EnvironmentMapLight>,
        Option<&ShadowFilteringMethod>,
        Option<&ScreenSpaceAmbientOcclusionSettings>,
        Has<NormalPrepass>,
        Has<DepthPrepass>,
        Has<MotionVectorPrepass>,
        Has<DeferredPrepass>,
        Option<&TemporalAntiAliasSettings>,
        Option<&Projection>,
    )>,
    mut commands: Commands,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (
        view_entity,
        view,
        tonemapping,
        dither,
        environment_map,
        shadow_filter_method,
        ssao,
        normal_prepass,
        depth_prepass,
        motion_vector_prepass,
        deferred_prepass,
        taa_settings,
        projection,
    ) in &views
    {
        let mut opaque_pass_material_map = Vec::new();
        let mut prepass_material_map = Vec::new();

        let mut view_key = MeshPipelineKey::from_msaa_samples(1);

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
            };
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }
        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }
        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }
        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        let environment_map_loaded = environment_map.is_some_and(|map| map.is_loaded(&images));
        if environment_map_loaded {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Castano13 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13;
            }
            ShadowFilteringMethod::Jimenez14 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14;
            }
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }

        if ssao.is_some() {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        if taa_settings.is_some() {
            view_key |= MeshPipelineKey::TAA;
        }

        for material_id in render_material_instances.values() {
            let Some(material) = render_materials.get(material_id) else {
                continue;
            };
            if material.properties.alpha_mode != AlphaMode::Opaque {
                continue;
            }

            let pipeline_descriptor = RenderPipelineDescriptor {
                label: todo!(),
                layout: todo!(),
                push_constant_ranges: todo!(),
                vertex: todo!(),
                primitive: todo!(),
                depth_stencil: todo!(),
                multisample: todo!(),
                fragment: todo!(),
            };

            let material_id = gpu_scene.get_material_id(material_id.untyped());

            let pipeline_id = *cache
                .entry(view_key)
                .or_insert_with(|| pipeline_cache.queue_render_pipeline(pipeline_descriptor));
            opaque_pass_material_map.push((material_id, pipeline_id, material.bind_group.clone()));

            let pipeline_id = *cache
                .entry(view_key)
                .or_insert_with(|| pipeline_cache.queue_render_pipeline(pipeline_descriptor));
            prepass_material_map.push((material_id, pipeline_id, material.bind_group.clone()));
        }

        commands.entity(view_entity).insert(MeshletViewMaterials {
            opaque_pass: opaque_pass_material_map,
            prepass: prepass_material_map,
        });
    }
}
