use std::{hash::Hash, marker::PhantomData};

use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    prelude::{FromWorld, Res, Resource, World},
    system::{ResMut, SystemParam},
};
use bevy_render::{
    mesh::MeshVertexBufferLayout,
    prelude::Mesh,
    render_asset::RenderAssets,
    render_resource::{
        BindGroupLayout, CachedRenderPipelineId, FragmentState, MultisampleState, PipelineCache,
        PipelineCacheError, RenderPipelineDescriptor, Shader, Source, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, SpecializedMeshPipelines, VertexState,
    },
    renderer::RenderDevice,
    view::Msaa,
    MainWorld,
};
use bevy_utils::HashMap;

use crate::{
    AlphaMode, Material, MaterialPipeline, MaterialPipelineKey, MeshPipelineKey, RenderMaterials,
    ShadowPipeline,
};

use thiserror::Error;

#[derive(Resource)]
pub struct DepthPipeline<M: Material> {
    material_pipeline: MaterialPipeline<M>,
}

pub struct DepthPipelineKey<M: Material> {
    params: Option<DepthPipelineSpecializationParams>,
    material_key: MaterialPipelineKey<M>,
}

impl<M: Material> Eq for DepthPipelineKey<M> where M::Data: PartialEq {}

impl<M: Material> PartialEq for DepthPipelineKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.material_key == other.material_key
    }
}

impl<M: Material> Clone for DepthPipelineKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            params: self
                .params
                .as_ref()
                .map(|params| DepthPipelineSpecializationParams {
                    view_layout: params.view_layout.clone(),
                    vertex_shader: params.vertex_shader.clone_weak(),
                    fragment_shader: params.fragment_shader.as_ref().map(Handle::clone_weak),
                }),
            material_key: self.material_key.clone(),
        }
    }
}

impl<M: Material> Hash for DepthPipelineKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.material_key.hash(state);
    }
}

struct DepthPipelineSpecializationParams {
    view_layout: BindGroupLayout,
    vertex_shader: Handle<Shader>,
    fragment_shader: Option<Handle<Shader>>,
}

impl<M: Material> SpecializedMeshPipeline for DepthPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = DepthPipelineKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let DepthPipelineSpecializationParams {
            view_layout,
            vertex_shader,
            fragment_shader,
        } = key.params.unwrap();

        let mut descriptor = self
            .material_pipeline
            .specialize(key.material_key, layout)?;

        if let Some(fragment_shader) = fragment_shader {
            let fragment_state = descriptor.fragment.as_mut().unwrap();
            fragment_state.shader = fragment_shader;
            fragment_state.targets.clear();
        } else {
            descriptor.fragment = None;
        }

        descriptor.vertex.shader = vertex_shader;

        descriptor.layout.as_mut().unwrap()[0] = view_layout;
        descriptor.multisample = MultisampleState::default();

        Ok(descriptor)
    }
}

impl<M: Material> FromWorld for DepthPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let material_pipeline = MaterialPipeline::from_world(world);

        DepthPipeline { material_pipeline }
    }
}

#[derive(SystemParam)]
pub struct DepthPipelineBuilder<'w, 's, M: Material>
where
    <M as bevy_render::render_resource::AsBindGroup>::Data: Eq + Hash + Clone,
{
    render_device: Res<'w, RenderDevice>,
    depth_pipeline: Res<'w, DepthPipeline<M>>,
    depth_pipelines: ResMut<'w, SpecializedMeshPipelines<DepthPipeline<M>>>,
    material_pipeline: Res<'w, MaterialPipeline<M>>,
    material_pipelines: ResMut<'w, SpecializedMeshPipelines<MaterialPipeline<M>>>,
    shadow_pipeline: Res<'w, ShadowPipeline>,
    pipeline_cache: ResMut<'w, PipelineCache>,
    render_meshes: Res<'w, RenderAssets<Mesh>>,
    render_materials: Res<'w, RenderMaterials<M>>,
    derived_shaders: ResMut<'w, DerivedShaders<M>>,
    msaa: Res<'w, Msaa>,
    #[system_param(ignore)]
    _p: PhantomData<&'s ()>,
}

#[derive(Error, Debug)]
pub enum DepthPipelineError {
    #[error(transparent)]
    MaterialError(#[from] SpecializedMeshPipelineError),
    #[error(transparent)]
    PipelineCacheError(#[from] PipelineCacheError),
    #[error("source is not compiled to naga")]
    NotNagaError,
    #[error("maybe next time")]
    Retry,
}

impl<'w, 's, M: Material> DepthPipelineBuilder<'w, 's, M>
where
    <M as bevy_render::render_resource::AsBindGroup>::Data: Eq + Hash + Clone,
{
    pub fn depth_pipeline_id(
        &mut self,
        material_handle: &Handle<M>,
        mesh_handle: &Handle<Mesh>,
    ) -> Result<CachedRenderPipelineId, DepthPipelineError> {
        let DepthPipelineBuilder {
            render_device,
            depth_pipelines,
            depth_pipeline,
            material_pipelines,
            material_pipeline,
            shadow_pipeline,
            pipeline_cache,
            render_meshes,
            render_materials,
            derived_shaders,
            msaa,
            ..
        } = self;

        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        if let Some(material) = render_materials.get(material_handle) {
            if let Some(mesh) = render_meshes.get(mesh_handle) {
                let mut mesh_key =
                    MeshPipelineKey::from_primitive_topology(mesh.primitive_topology) | msaa_key;
                let alpha_mode = material.properties.alpha_mode;
                if let AlphaMode::Blend = alpha_mode {
                    mesh_key |= MeshPipelineKey::TRANSPARENT_MAIN_PASS;
                }

                let depth_key = DepthPipelineKey {
                    params: None,
                    material_key: MaterialPipelineKey {
                        mesh_key,
                        bind_group_data: material.key.clone(),
                    },
                };

                // check for existing pipeline
                if let Some(pipeline_id) = depth_pipelines.get(&depth_key, &mesh.layout) {
                    return Ok(pipeline_id);
                }

                // check for pre-built shaders
                if let Some(mut handles) = derived_shaders.added.remove(&DerivedShaderHandle {
                    key: depth_key.material_key.clone(),
                }) {
                    let depth_key = DepthPipelineKey {
                        params: Some(DepthPipelineSpecializationParams {
                            view_layout: shadow_pipeline.view_layout.clone(),
                            vertex_shader: handles.pop().unwrap(),
                            fragment_shader: handles.pop(),
                        }),
                        material_key: depth_key.material_key,
                    };

                    return Ok(depth_pipelines.specialize(
                        pipeline_cache,
                        depth_pipeline,
                        depth_key,
                        &mesh.layout,
                    )?);
                }

                // build shaders for next time
                let mut shaders_to_queue = Vec::default();

                let material_pipeline_id = material_pipelines.specialize(
                    pipeline_cache,
                    material_pipeline,
                    depth_key.material_key.clone(),
                    &mesh.layout,
                )?;

                let material_pipeline_descriptor =
                    pipeline_cache.get_render_pipeline_descriptor(material_pipeline_id);
                let VertexState {
                    shader: vertex_shader,
                    entry_point: vertex_entry_point,
                    shader_defs: vertex_shader_defs,
                    ..
                } = material_pipeline_descriptor.vertex.clone();
                let mut required_frag_bindings = None;

                if let Some(FragmentState {
                    shader: fragment_shader,
                    entry_point: fragment_entry_point,
                    shader_defs: fragment_shader_defs,
                    ..
                }) = material_pipeline_descriptor.fragment.clone()
                {
                    let naga_module = pipeline_cache.get_shader_source(
                        render_device,
                        material_pipeline_id,
                        &fragment_shader,
                        &fragment_shader_defs,
                    )?;

                    match naga_module {
                        None => return Err(DepthPipelineError::NotNagaError),
                        Some(fragment_module) => {
                            let mut pruner = naga_oil::prune::Pruner::new(fragment_module);
                            let fragment_entry_point = fragment_module
                                .entry_points
                                .iter()
                                .find(|ep| ep.name.as_str() == fragment_entry_point)
                                .unwrap();

                            let inputs = pruner.add_entrypoint(
                                fragment_entry_point,
                                Default::default(),
                                None,
                            );

                            if !inputs.is_empty() {
                                let fragment_depth_shader = pruner.rewrite();

                                shaders_to_queue.push(Shader {
                                    source: Source::Naga(Box::new(fragment_depth_shader)),
                                    path: None,
                                    import_path: None,
                                    imports: Default::default(),
                                    additional_imports: Default::default(),
                                });
                                required_frag_bindings = Some(inputs.input_to_bindings(
                                    fragment_module,
                                    &fragment_entry_point.function,
                                ));
                            }
                        }
                    }
                }

                let naga_module = pipeline_cache.get_shader_source(
                    render_device,
                    material_pipeline_id,
                    &vertex_shader,
                    &vertex_shader_defs,
                )?;

                match naga_module {
                    None => return Err(DepthPipelineError::NotNagaError),
                    Some(vertex_module) => {
                        let vertex_entrypoint = vertex_module
                            .entry_points
                            .iter()
                            .find(|ep| ep.name.as_str() == vertex_entry_point)
                            .unwrap();

                        let mut requirements = match required_frag_bindings {
                            Some(bindings) => {
                                naga_oil::prune::RequiredContext::output_from_bindings(
                                    &bindings,
                                    vertex_module,
                                    &vertex_entrypoint.function,
                                )
                            }
                            None => naga_oil::prune::RequiredContext::default(),
                        };

                        // note: add both forms of position to get whatever is available
                        requirements.add_binding(
                            naga_oil::prune::Binding::BuiltIn(naga_oil::prune::BuiltIn::Position {
                                invariant: true,
                            }),
                            vertex_module,
                            &vertex_entrypoint.function,
                        );
                        requirements.add_binding(
                            naga_oil::prune::Binding::BuiltIn(naga_oil::prune::BuiltIn::Position {
                                invariant: false,
                            }),
                            vertex_module,
                            &vertex_entrypoint.function,
                        );

                        let mut pruner = naga_oil::prune::Pruner::new(vertex_module);
                        pruner.add_entrypoint(
                            vertex_entrypoint,
                            requirements.globals,
                            requirements.retval,
                        );

                        let vertex_shader = pruner.rewrite();

                        shaders_to_queue.push(Shader {
                            source: Source::Naga(Box::new(vertex_shader)),
                            path: None,
                            import_path: None,
                            imports: Default::default(),
                            additional_imports: Default::default(),
                        });
                    }
                }

                derived_shaders.to_add.insert(
                    DerivedShaderHandle {
                        key: depth_key.material_key,
                    },
                    shaders_to_queue,
                );
                return Err(DepthPipelineError::Retry);
            }
        }

        // mesh and/or material missing
        Err(DepthPipelineError::Retry)
    }
}

pub struct DerivedShaderHandle<M: Material> {
    key: MaterialPipelineKey<M>,
}

impl<M: Material> Eq for DerivedShaderHandle<M> where M::Data: PartialEq {}

impl<M: Material> PartialEq for DerivedShaderHandle<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<M: Material> Clone for DerivedShaderHandle<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
        }
    }
}

impl<M: Material> Hash for DerivedShaderHandle<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

#[derive(Resource)]
pub struct DerivedShaders<M: Material> {
    to_add: HashMap<DerivedShaderHandle<M>, Vec<Shader>>,
    added: HashMap<DerivedShaderHandle<M>, Vec<Handle<Shader>>>,
}

impl<M: Material> FromWorld for DerivedShaders<M> {
    fn from_world(_: &mut World) -> Self {
        Self {
            to_add: Default::default(),
            added: Default::default(),
        }
    }
}

pub fn unextract_derived_shaders<M: Material>(
    mut world: ResMut<MainWorld>,
    mut derived_shaders: ResMut<DerivedShaders<M>>,
) where
    <M as bevy_render::render_resource::AsBindGroup>::Data: Eq + Hash + Clone,
{
    let mut shaders = world.resource_mut::<Assets<Shader>>();

    for (derived_handle, shaders_to_add) in std::mem::take(&mut derived_shaders.to_add) {
        let shader_handles = shaders_to_add
            .into_iter()
            .map(|shader| shaders.add(shader))
            .collect();
        derived_shaders.added.insert(derived_handle, shader_handles);
    }
}
