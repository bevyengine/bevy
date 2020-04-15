use super::{
    BindType, PipelineDescriptor, PipelineLayout, PipelineLayoutType, VertexBufferDescriptors,
};
use crate::{
    render_resource::{
        BufferInfo, RenderResourceAssignments, RenderResourceAssignmentsId, ResourceInfo,
    },
    renderer_2::RenderContext,
    shader::{Shader, ShaderSource},
    Renderable,
};
use bevy_asset::{AssetStorage, Handle};
use std::collections::{HashMap, HashSet};

use legion::prelude::*;

// TODO: consider using (Typeid, fieldinfo.index) in place of string for hashes
pub struct PipelineCompiler {
    pub shader_source_to_compiled: HashMap<Handle<Shader>, Vec<(HashSet<String>, Handle<Shader>)>>,
    pub pipeline_source_to_compiled:
        HashMap<Handle<PipelineDescriptor>, Vec<(HashSet<String>, Handle<PipelineDescriptor>)>>,
}

impl PipelineCompiler {
    pub fn new() -> Self {
        PipelineCompiler {
            shader_source_to_compiled: HashMap::new(),
            pipeline_source_to_compiled: HashMap::new(),
        }
    }

    fn reflect_layout(
        shader_storage: &AssetStorage<Shader>,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        pipeline_descriptor: &mut PipelineDescriptor,
        render_context: &dyn RenderContext,
        render_resource_assignments: &RenderResourceAssignments,
    ) {
        let vertex_spirv = shader_storage
            .get(&pipeline_descriptor.shader_stages.vertex)
            .unwrap();
        let fragment_spirv = pipeline_descriptor
            .shader_stages
            .fragment
            .as_ref()
            .map(|handle| &*shader_storage.get(&handle).unwrap());

        let mut layouts = vec![vertex_spirv.reflect_layout().unwrap()];
        if let Some(ref fragment_spirv) = fragment_spirv {
            layouts.push(fragment_spirv.reflect_layout().unwrap());
        }

        let mut layout = PipelineLayout::from_shader_layouts(&mut layouts);
        layout.sync_vertex_buffer_descriptors(vertex_buffer_descriptors);

        // set binding uniforms to dynamic if render resource assignments use dynamic
        // TODO: this breaks down if different assignments have different "dynamic" status or if the dynamic status changes.
        // the fix would be to add "dynamic bindings" to the existing shader_def sets. this would ensure new pipelines are generated
        // for all permutations of dynamic/non-dynamic
        for bind_group in layout.bind_groups.iter_mut() {
            for binding in bind_group.bindings.iter_mut() {
                if let Some(render_resource) = render_resource_assignments.get(&binding.name) {
                    if let Some(ResourceInfo::Buffer(BufferInfo { is_dynamic, .. })) =
                        render_context
                            .resources()
                            .get_resource_info(render_resource)
                    {
                        if let BindType::Uniform {
                            ref mut dynamic, ..
                        } = binding.bind_type
                        {
                            *dynamic = *is_dynamic
                        }
                    }
                }
            }
        }

        pipeline_descriptor.layout = PipelineLayoutType::Reflected(Some(layout));
    }

    fn compile_shader(
        &mut self,
        shader_storage: &mut AssetStorage<Shader>,
        shader_handle: &Handle<Shader>,
        shader_defs: &HashSet<String>,
    ) -> Handle<Shader> {
        let compiled_shaders = self
            .shader_source_to_compiled
            .entry(*shader_handle)
            .or_insert_with(|| Vec::new());

        let shader = shader_storage.get(shader_handle).unwrap();

        // don't produce new shader if the input source is already spirv
        if let ShaderSource::Spirv(_) = shader.source {
            return *shader_handle;
        }

        if let Some((_compiled_shader_defs, compiled_shader)) = compiled_shaders
            .iter()
            .find(|(compiled_shader_defs, _compiled_shader)| *compiled_shader_defs == *shader_defs)
        {
            // if shader has already been compiled with current configuration, use existing shader
            *compiled_shader
        } else {
            // if no shader exists with the current configuration, create new shader and compile
            let shader_def_vec = shader_defs.iter().cloned().collect::<Vec<String>>();
            let compiled_shader = shader.get_spirv_shader(Some(&shader_def_vec));
            compiled_shaders.push((shader_defs.clone(), *shader_handle));
            shader_storage.add(compiled_shader)
        }
    }

    fn compile_pipeline(
        &mut self,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        shader_storage: &mut AssetStorage<Shader>,
        render_context: &dyn RenderContext,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> PipelineDescriptor {
        let mut compiled_pipeline_descriptor = pipeline_descriptor.clone();

        compiled_pipeline_descriptor.shader_stages.vertex = self.compile_shader(
            shader_storage,
            &pipeline_descriptor.shader_stages.vertex,
            &render_resource_assignments.shader_defs,
        );
        compiled_pipeline_descriptor.shader_stages.fragment = pipeline_descriptor
            .shader_stages
            .fragment
            .as_ref()
            .map(|fragment| {
                self.compile_shader(
                    shader_storage,
                    fragment,
                    &render_resource_assignments.shader_defs,
                )
            });

        Self::reflect_layout(
            shader_storage,
            vertex_buffer_descriptors,
            &mut compiled_pipeline_descriptor,
            render_context,
            render_resource_assignments,
        );

        compiled_pipeline_descriptor
    }

    fn update_shader_assignments(
        &mut self,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        shader_pipeline_assignments: &mut PipelineAssignments,
        render_context: &dyn RenderContext,
        pipeline_storage: &mut AssetStorage<PipelineDescriptor>,
        shader_storage: &mut AssetStorage<Shader>,
        pipelines: &[Handle<PipelineDescriptor>],
        render_resource_assignments: &RenderResourceAssignments,
    ) {
        for pipeline_handle in pipelines.iter() {
            if let None = self.pipeline_source_to_compiled.get(pipeline_handle) {
                self.pipeline_source_to_compiled
                    .insert(*pipeline_handle, Vec::new());
            }

            let final_handle = if let Some((_shader_defs, macroed_pipeline_handle)) = self
                .pipeline_source_to_compiled
                .get_mut(pipeline_handle)
                .unwrap()
                .iter()
                .find(|(shader_defs, _macroed_pipeline_handle)| {
                    *shader_defs == render_resource_assignments.shader_defs
                }) {
                *macroed_pipeline_handle
            } else {
                let pipeline_descriptor = pipeline_storage.get(pipeline_handle).unwrap();
                let compiled_pipeline = self.compile_pipeline(
                    vertex_buffer_descriptors,
                    shader_storage,
                    render_context,
                    pipeline_descriptor,
                    render_resource_assignments,
                );
                let compiled_pipeline_handle = pipeline_storage.add(compiled_pipeline);

                let macro_pipelines = self
                    .pipeline_source_to_compiled
                    .get_mut(pipeline_handle)
                    .unwrap();
                macro_pipelines.push((
                    render_resource_assignments.shader_defs.clone(),
                    compiled_pipeline_handle,
                ));
                compiled_pipeline_handle
            };

            // TODO: this will break down if pipeline layout changes. fix this with "auto-layout"
            if let None = shader_pipeline_assignments.assignments.get(&final_handle) {
                shader_pipeline_assignments
                    .assignments
                    .insert(final_handle, Vec::new());
            }

            let assignments = shader_pipeline_assignments
                .assignments
                .get_mut(&final_handle)
                .unwrap();
            assignments.push(render_resource_assignments.id);
        }
    }

    pub fn iter_compiled_pipelines(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<PipelineDescriptor>>> {
        if let Some(compiled_pipelines) = self.pipeline_source_to_compiled.get(&pipeline_handle) {
            Some(compiled_pipelines.iter().map(|(_, handle)| handle))
        } else {
            None
        }
    }
}

pub struct PipelineAssignments {
    pub assignments: HashMap<Handle<PipelineDescriptor>, Vec<RenderResourceAssignmentsId>>,
}

impl PipelineAssignments {
    pub fn new() -> Self {
        PipelineAssignments {
            assignments: HashMap::new(),
        }
    }
}

// TODO: make this a system
pub fn update_shader_assignments(
    world: &mut World,
    resources: &Resources,
    render_context: &dyn RenderContext,
) {
    // PERF: this seems like a lot of work for things that don't change that often.
    // lots of string + hashset allocations. sees uniform_resource_provider for more context
    {
        let mut shader_pipeline_assignments = resources.get_mut::<PipelineAssignments>().unwrap();
        let mut pipeline_compiler = resources.get_mut::<PipelineCompiler>().unwrap();
        let mut shader_storage = resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let vertex_buffer_descriptors = resources.get::<VertexBufferDescriptors>().unwrap();
        let mut pipeline_descriptor_storage = resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();

        // reset assignments so they are updated every frame
        shader_pipeline_assignments.assignments = HashMap::new();

        // TODO: only update when renderable is changed
        for mut renderable in <Write<Renderable>>::query().iter_mut(world) {
            // skip instanced entities. their batched RenderResourceAssignments will handle shader assignments
            if renderable.is_instanced {
                continue;
            }

            pipeline_compiler.update_shader_assignments(
                &vertex_buffer_descriptors,
                &mut shader_pipeline_assignments,
                render_context,
                &mut pipeline_descriptor_storage,
                &mut shader_storage,
                &renderable.pipelines,
                &renderable.render_resource_assignments,
            );

            // reset shader_defs so they can be changed next frame
            renderable.render_resource_assignments.shader_defs.clear();
        }
    }
}
