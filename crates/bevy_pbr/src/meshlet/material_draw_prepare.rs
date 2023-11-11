use super::{MeshletGpuScene, MESHLET_MESH_MATERIAL_SHADER_HANDLE};
use crate::*;
use bevy_asset::AssetServer;
use bevy_core_pipeline::{
    core_3d::Camera3d,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_render::{
    camera::{Projection, TemporalJitter},
    mesh::{InnerMeshVertexBufferLayout, Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_resource::*,
    texture::BevyDefault,
    view::{ExtractedView, ViewTarget},
};
use bevy_utils::HashMap;
use std::hash::Hash;

#[derive(Component)]
pub struct MeshletViewMaterials {
    pub opaque_pass: Vec<(u32, CachedRenderPipelineId, BindGroup)>,
    pub prepass: Vec<(u32, CachedRenderPipelineId, BindGroup)>,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_material_meshlet_meshes<M: Material>(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    mut cache: Local<HashMap<MeshPipelineKey, CachedRenderPipelineId>>,
    pipeline_cache: Res<PipelineCache>,
    material_pipeline: Res<MaterialPipeline<M>>,
    mesh_pipeline: Res<MeshPipeline>,
    render_materials: Res<RenderMaterials<M>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    images: Res<RenderAssets<Image>>,
    asset_server: Res<AssetServer>,
    views: Query<(
        Entity,
        &ExtractedView,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&EnvironmentMapLight>,
        Option<&ShadowFilteringMethod>,
        Has<ScreenSpaceAmbientOcclusionSettings>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        Option<&Camera3d>,
        Has<TemporalJitter>,
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
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
        camera_3d,
        temporal_jitter,
        projection,
    ) in &views
    {
        let mut opaque_pass_material_map = Vec::new();
        let mut prepass_material_map = Vec::new();

        let fake_vertex_buffer_layout =
            &MeshVertexBufferLayout::new(InnerMeshVertexBufferLayout::new(
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
            ));

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

        let environment_map_loaded = environment_map.is_some_and(|map| map.is_loaded(&images));

        if environment_map_loaded {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
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
        if ssao {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        if let Some(camera_3d) = camera_3d {
            view_key |= screen_space_specular_transmission_pipeline_key(
                camera_3d.screen_space_specular_transmission_quality,
            );
        }

        view_key |= MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);

        for material_id in render_material_instances.values() {
            let Some(material) = render_materials.get(material_id) else {
                continue;
            };
            if material.properties.alpha_mode != AlphaMode::Opaque {
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

            let mut shader_defs = material_pipeline_descriptor
                .fragment
                .expect("TODO")
                .shader_defs;
            shader_defs.extend_from_slice(&[
                "MESHLET_MESH_MATERIAL_PASS".into(),
                ShaderDefVal::UInt("MESHLET_BIND_GROUP".into(), 1),
                "MATERIAL_BIND_GROUP_2".into(),
            ]);

            let pipeline_descriptor = RenderPipelineDescriptor {
                label: Some("meshlet_material_draw".into()),
                layout: vec![
                    mesh_pipeline.get_view_layout(view_key.into()).clone(),
                    gpu_scene.material_draw_bind_group_layout(),
                    material_pipeline.material_layout.clone(),
                ],
                push_constant_ranges: vec![],
                vertex: VertexState {
                    shader: MESHLET_MESH_MATERIAL_SHADER_HANDLE,
                    shader_defs: shader_defs.clone(),
                    entry_point: "vertex".into(),
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
                    entry_point: "fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: if view.hdr {
                            ViewTarget::TEXTURE_FORMAT_HDR
                        } else {
                            TextureFormat::bevy_default()
                        },
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
            };

            let material_id = gpu_scene.get_material_id(material_id.untyped());

            let pipeline_id = *cache.entry(view_key).or_insert_with(|| {
                pipeline_cache.queue_render_pipeline(pipeline_descriptor.clone())
            });
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
