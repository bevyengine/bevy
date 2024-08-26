use super::{MeshletGpuScene, MESHLET_MESH_MATERIAL_SHADER_HANDLE};
use crate::{environment_map::EnvironmentMapLight, irradiance_volume::IrradianceVolume, *};
use bevy_asset::AssetServer;
use bevy_core_pipeline::{
    core_3d::Camera3d,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_derive::{Deref, DerefMut};
use bevy_render::{
    camera::TemporalJitter,
    mesh::{Mesh, MeshVertexBufferLayout, MeshVertexBufferLayoutRef, MeshVertexBufferLayouts},
    render_asset::RenderAssets,
    render_resource::*,
    view::ExtractedView,
};
use bevy_utils::{HashMap, HashSet};
use std::hash::Hash;

/// A list of `(Material ID, Pipeline, BindGroup)` for a view for use in [`super::MeshletMainOpaquePass3dNode`].
#[derive(Component, Deref, DerefMut, Default)]
pub struct MeshletViewMaterialsMainOpaquePass(pub Vec<(u32, CachedRenderPipelineId, BindGroup)>);

/// Prepare [`Material`] pipelines for [`super::MeshletMesh`] entities for use in [`super::MeshletMainOpaquePass3dNode`],
/// and register the material with [`MeshletGpuScene`].
#[allow(clippy::too_many_arguments)]
pub fn prepare_material_meshlet_meshes_main_opaque_pass<M: Material>(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    mut cache: Local<HashMap<MeshPipelineKey, CachedRenderPipelineId>>,
    pipeline_cache: Res<PipelineCache>,
    material_pipeline: Res<MaterialPipeline<M>>,
    mesh_pipeline: Res<MeshPipeline>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    asset_server: Res<AssetServer>,
    mut mesh_vertex_buffer_layouts: ResMut<MeshVertexBufferLayouts>,
    mut views: Query<
        (
            &mut MeshletViewMaterialsMainOpaquePass,
            &ExtractedView,
            Option<&Tonemapping>,
            Option<&DebandDither>,
            Option<&ShadowFilteringMethod>,
            Has<ScreenSpaceAmbientOcclusionSettings>,
            (
                Has<NormalPrepass>,
                Has<DepthPrepass>,
                Has<MotionVectorPrepass>,
                Has<DeferredPrepass>,
            ),
            Has<TemporalJitter>,
            Option<&Projection>,
            Has<RenderViewLightProbes<EnvironmentMapLight>>,
            Has<RenderViewLightProbes<IrradianceVolume>>,
        ),
        With<Camera3d>,
    >,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let fake_vertex_buffer_layout = &fake_vertex_buffer_layout(&mut mesh_vertex_buffer_layouts);

    for (
        mut materials,
        view,
        tonemapping,
        dither,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
        temporal_jitter,
        projection,
        has_environment_maps,
        has_irradiance_volumes,
    ) in &mut views
    {
        let mut view_key =
            MeshPipelineKey::from_msaa_samples(1) | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }
        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }
        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }
        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        if temporal_jitter {
            view_key |= MeshPipelineKey::TEMPORAL_JITTER;
        }

        if has_environment_maps {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        if has_irradiance_volumes {
            view_key |= MeshPipelineKey::IRRADIANCE_VOLUME;
        }

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
            };
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Gaussian => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_GAUSSIAN;
            }
            ShadowFilteringMethod::Temporal => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_TEMPORAL;
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

        if ssao {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        view_key |= MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);

        for material_id in render_material_instances.values().collect::<HashSet<_>>() {
            let Some(material) = render_materials.get(*material_id) else {
                continue;
            };

            if material.properties.render_method != OpaqueRendererMethod::Forward
                || material.properties.alpha_mode != AlphaMode::Opaque
                || material.properties.reads_view_transmission_texture
            {
                continue;
            }

            let Ok(material_pipeline_descriptor) = material_pipeline.specialize(
                MaterialPipelineKey {
                    mesh_key: view_key,
                    bind_group_data: material.key.clone(),
                },
                fake_vertex_buffer_layout,
            ) else {
                continue;
            };
            let material_fragment = material_pipeline_descriptor.fragment.unwrap();

            let mut shader_defs = material_fragment.shader_defs;
            shader_defs.push("MESHLET_MESH_MATERIAL_PASS".into());

            let pipeline_descriptor = RenderPipelineDescriptor {
                label: material_pipeline_descriptor.label,
                layout: vec![
                    mesh_pipeline.get_view_layout(view_key.into()).clone(),
                    gpu_scene.material_draw_bind_group_layout(),
                    material_pipeline.material_layout.clone(),
                ],
                push_constant_ranges: vec![],
                vertex: VertexState {
                    shader: MESHLET_MESH_MATERIAL_SHADER_HANDLE,
                    shader_defs: shader_defs.clone(),
                    entry_point: material_pipeline_descriptor.vertex.entry_point,
                    buffers: Vec::new(),
                },
                primitive: PrimitiveState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth16Unorm,
                    depth_write_enabled: false,
                    depth_compare: CompareFunction::Equal,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    shader: match M::meshlet_mesh_fragment_shader() {
                        ShaderRef::Default => MESHLET_MESH_MATERIAL_SHADER_HANDLE,
                        ShaderRef::Handle(handle) => handle,
                        ShaderRef::Path(path) => asset_server.load(path),
                    },
                    shader_defs,
                    entry_point: material_fragment.entry_point,
                    targets: material_fragment.targets,
                }),
            };

            let material_id = gpu_scene.get_material_id(material_id.untyped());

            let pipeline_id = *cache.entry(view_key).or_insert_with(|| {
                pipeline_cache.queue_render_pipeline(pipeline_descriptor.clone())
            });
            materials.push((material_id, pipeline_id, material.bind_group.clone()));
        }
    }
}

/// A list of `(Material ID, Pipeline, BindGroup)` for a view for use in [`super::MeshletPrepassNode`].
#[derive(Component, Deref, DerefMut, Default)]
pub struct MeshletViewMaterialsPrepass(pub Vec<(u32, CachedRenderPipelineId, BindGroup)>);

/// A list of `(Material ID, Pipeline, BindGroup)` for a view for use in [`super::MeshletDeferredGBufferPrepassNode`].
#[derive(Component, Deref, DerefMut, Default)]
pub struct MeshletViewMaterialsDeferredGBufferPrepass(
    pub Vec<(u32, CachedRenderPipelineId, BindGroup)>,
);

/// Prepare [`Material`] pipelines for [`super::MeshletMesh`] entities for use in [`super::MeshletPrepassNode`],
/// and [`super::MeshletDeferredGBufferPrepassNode`] and register the material with [`MeshletGpuScene`].
#[allow(clippy::too_many_arguments)]
pub fn prepare_material_meshlet_meshes_prepass<M: Material>(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    mut cache: Local<HashMap<MeshPipelineKey, CachedRenderPipelineId>>,
    pipeline_cache: Res<PipelineCache>,
    prepass_pipeline: Res<PrepassPipeline<M>>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    mut mesh_vertex_buffer_layouts: ResMut<MeshVertexBufferLayouts>,
    asset_server: Res<AssetServer>,
    mut views: Query<
        (
            &mut MeshletViewMaterialsPrepass,
            &mut MeshletViewMaterialsDeferredGBufferPrepass,
            &ExtractedView,
            AnyOf<(&NormalPrepass, &MotionVectorPrepass, &DeferredPrepass)>,
        ),
        With<Camera3d>,
    >,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let fake_vertex_buffer_layout = &fake_vertex_buffer_layout(&mut mesh_vertex_buffer_layouts);

    for (
        mut materials,
        mut deferred_materials,
        view,
        (normal_prepass, motion_vector_prepass, deferred_prepass),
    ) in &mut views
    {
        let mut view_key =
            MeshPipelineKey::from_msaa_samples(1) | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass.is_some() {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }
        if motion_vector_prepass.is_some() {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        view_key |= MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);

        for material_id in render_material_instances.values().collect::<HashSet<_>>() {
            let Some(material) = render_materials.get(*material_id) else {
                continue;
            };

            if material.properties.alpha_mode != AlphaMode::Opaque
                || material.properties.reads_view_transmission_texture
            {
                continue;
            }

            let material_wants_deferred = matches!(
                material.properties.render_method,
                OpaqueRendererMethod::Deferred
            );
            if deferred_prepass.is_some() && material_wants_deferred {
                view_key |= MeshPipelineKey::DEFERRED_PREPASS;
            } else if normal_prepass.is_none() && motion_vector_prepass.is_none() {
                continue;
            }

            let Ok(material_pipeline_descriptor) = prepass_pipeline.specialize(
                MaterialPipelineKey {
                    mesh_key: view_key,
                    bind_group_data: material.key.clone(),
                },
                fake_vertex_buffer_layout,
            ) else {
                continue;
            };
            let material_fragment = material_pipeline_descriptor.fragment.unwrap();

            let mut shader_defs = material_fragment.shader_defs;
            shader_defs.push("MESHLET_MESH_MATERIAL_PASS".into());

            let view_layout = if view_key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
                prepass_pipeline.view_layout_motion_vectors.clone()
            } else {
                prepass_pipeline.view_layout_no_motion_vectors.clone()
            };

            let fragment_shader = if view_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
                M::meshlet_mesh_deferred_fragment_shader()
            } else {
                M::meshlet_mesh_prepass_fragment_shader()
            };

            let entry_point = match fragment_shader {
                ShaderRef::Default => "prepass_fragment".into(),
                _ => material_fragment.entry_point,
            };

            let pipeline_descriptor = RenderPipelineDescriptor {
                label: material_pipeline_descriptor.label,
                layout: vec![
                    view_layout,
                    gpu_scene.material_draw_bind_group_layout(),
                    prepass_pipeline.material_layout.clone(),
                ],
                push_constant_ranges: vec![],
                vertex: VertexState {
                    shader: MESHLET_MESH_MATERIAL_SHADER_HANDLE,
                    shader_defs: shader_defs.clone(),
                    entry_point: material_pipeline_descriptor.vertex.entry_point,
                    buffers: Vec::new(),
                },
                primitive: PrimitiveState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth16Unorm,
                    depth_write_enabled: false,
                    depth_compare: CompareFunction::Equal,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    shader: match fragment_shader {
                        ShaderRef::Default => MESHLET_MESH_MATERIAL_SHADER_HANDLE,
                        ShaderRef::Handle(handle) => handle,
                        ShaderRef::Path(path) => asset_server.load(path),
                    },
                    shader_defs,
                    entry_point,
                    targets: material_fragment.targets,
                }),
            };

            let material_id = gpu_scene.get_material_id(material_id.untyped());

            let pipeline_id = *cache.entry(view_key).or_insert_with(|| {
                pipeline_cache.queue_render_pipeline(pipeline_descriptor.clone())
            });

            let item = (material_id, pipeline_id, material.bind_group.clone());
            if view_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
                deferred_materials.push(item);
            } else {
                materials.push(item);
            }
        }
    }
}

// Meshlet materials don't use a traditional vertex buffer, but the material specialization requires one.
fn fake_vertex_buffer_layout(layouts: &mut MeshVertexBufferLayouts) -> MeshVertexBufferLayoutRef {
    layouts.insert(MeshVertexBufferLayout::new(
        vec![
            Mesh::ATTRIBUTE_POSITION.id,
            Mesh::ATTRIBUTE_NORMAL.id,
            Mesh::ATTRIBUTE_UV_0.id,
            Mesh::ATTRIBUTE_TANGENT.id,
        ],
        VertexBufferLayout {
            array_stride: 48,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_POSITION.format,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_NORMAL.format,
                    offset: 12,
                    shader_location: 1,
                },
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_UV_0.format,
                    offset: 24,
                    shader_location: 2,
                },
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_TANGENT.format,
                    offset: 32,
                    shader_location: 3,
                },
            ],
        },
    ))
}
