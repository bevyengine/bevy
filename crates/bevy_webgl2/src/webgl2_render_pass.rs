use crate::renderer::WebGL2RenderContext;
use crate::{gl_call, renderer::*, Buffer, WebGL2Pipeline};
use bevy_asset::Handle;
use bevy_render::{
    pass::RenderPass,
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    renderer::{BindGroupId, BufferId, RenderContext},
};
use std::ops::Range;
pub struct WebGL2RenderPass<'a> {
    pub render_context: &'a WebGL2RenderContext,
    pub pipeline_descriptor: Option<&'a PipelineDescriptor>,
    pub pipeline: Option<Handle<PipelineDescriptor>>,
}

impl<'a> WebGL2RenderPass<'a> {
    pub fn setup_vao(&self) {
        let gl = &self.render_context.device.get_context();

        let resources = &self.render_context.render_resource_context.resources;
        let mut pipelines = resources.pipelines.write();
        let pipeline_handle = self.pipeline.as_ref().unwrap();
        let pipeline = pipelines.get_mut(&pipeline_handle).unwrap();

        gl_call!(gl.bind_vertex_array(Some(&pipeline.vao)));

        if !pipeline.update_vao {
            return;
        }
        pipeline.update_vao = false;
        let buffers = resources.buffers.write();

        if let Some(buffer_id) = pipeline.vertex_buffer {
            let buffer = buffers.get(&buffer_id).unwrap();
            if let Buffer::WebGlBuffer(buffer_id) = &buffer.buffer {
                gl_call!(gl.bind_buffer(Gl::ARRAY_BUFFER, Some(buffer_id)));
            } else {
                panic!("binding in-memory buffer");
            }
        }

        if let Some(buffer_id) = pipeline.index_buffer {
            let buffer = buffers.get(&buffer_id).unwrap();
            if let Buffer::WebGlBuffer(buffer_id) = &buffer.buffer {
                gl_call!(gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(buffer_id)));
            } else {
                panic!("binding in-memory buffer");
            }
        }

        assert!(pipeline.vertex_buffer_descriptors.len() == 1);
        let vertex_buffer_descriptor = &pipeline.vertex_buffer_descriptors[0];
        log::info!(
            "stride: {:?}, num_of_attrs: {:?}",
            vertex_buffer_descriptor.stride,
            vertex_buffer_descriptor.attributes.len()
        );
        // log::info!("vertex_buffer_descriptor: {:?}", vertex_buffer_descriptor);
        for attr_descr in vertex_buffer_descriptor.attributes.iter() {
            log::info!(
                "attr_loc: {:?}, nr_of_comp: {:?}, format: {:?}, norm: {:?}, offset: {:?}",
                attr_descr.attrib_location,
                attr_descr.format.nr_of_components,
                attr_descr.format.format,
                attr_descr.format.normalized,
                attr_descr.offset
            );
            if attr_descr.attrib_location >= 0 {
                gl_call!(gl.enable_vertex_attrib_array(attr_descr.attrib_location as u32 as u32));
                gl_call!(gl.vertex_attrib_pointer_with_i32(
                    attr_descr.attrib_location as u32,
                    attr_descr.format.nr_of_components,
                    attr_descr.format.format,
                    attr_descr.format.normalized,
                    vertex_buffer_descriptor.stride,
                    attr_descr.offset,
                ));
            }
        }
    }
}

impl<'a> RenderPass for WebGL2RenderPass<'a> {
    fn get_render_context(&self) -> &dyn RenderContext {
        self.render_context
    }

    fn set_vertex_buffer(&mut self, _start_slot: u32, buffer_id: BufferId, _offset: u64) {
        // TODO - start_slot and offset parameters
        let resources = &self.render_context.render_resource_context.resources;
        let mut pipelines = resources.pipelines.write();
        let pipeline_handle = self.pipeline.as_ref().unwrap();
        let pipeline = pipelines.get_mut(&pipeline_handle).unwrap();

        if pipeline.vertex_buffer != Some(buffer_id) {
            pipeline.vertex_buffer = Some(buffer_id);
            pipeline.update_vao = true;
        }
    }

    fn set_viewport(
        &mut self,
        _x: f32,
        _y: f32,
        _w: f32,
        _h: f32,
        _min_depth: f32,
        _max_depth: f32,
    ) {
        // log::info!(
        //     "render_pass: set_viewport {:?}",
        //     (x, y, w, h, min_depth, max_depth)
        // );
        panic!("not implemented");
        // self.render_pass
        //     .set_viewport(x, y, w, h, min_depth, max_depth);
    }

    fn set_stencil_reference(&mut self, _reference: u32) {
        // log::info!("render_pass: set_stencil_reference {:?}", reference);
        //self.render_pass.set_stencil_reference(reference);
    }

    fn set_index_buffer(&mut self, buffer_id: BufferId, _offset: u64) {
        // TODO - offset parameter

        let resources = &self.render_context.render_resource_context.resources;
        let mut pipelines = resources.pipelines.write();
        let pipeline_handle = self.pipeline.as_ref().unwrap();
        let pipeline = pipelines.get_mut(&pipeline_handle).unwrap();

        if pipeline.index_buffer != Some(buffer_id) {
            pipeline.index_buffer = Some(buffer_id);
            pipeline.update_vao = true;
        }
    }

    fn draw_indexed(&mut self, indices: Range<u32>, _base_vertex: i32, _instances: Range<u32>) {
        let ctx = &self.render_context;
        let gl = &ctx.device.get_context();
        self.setup_vao();
        gl_call!(gl.draw_elements_with_i32(
            Gl::TRIANGLES,
            indices.end as i32,
            Gl::UNSIGNED_INT,
            indices.start as i32
        ));
        let gl_null = None;
        gl_call!(gl.bind_vertex_array(gl_null));
    }

    fn draw(&mut self, vertices: Range<u32>, _instances: Range<u32>) {
        let ctx = &self.render_context;
        let gl = &ctx.device.get_context();
        self.setup_vao();
        gl_call!(gl.draw_arrays(
            Gl::TRIANGLES,
            vertices.start as i32,
            (vertices.end - vertices.start) as i32
        ));
        let gl_null = None;
        gl_call!(gl.bind_vertex_array(gl_null));
    }

    fn set_bind_group(
        &mut self,
        _index: u32,
        _bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group_id: BindGroupId,
        dynamic_uniform_indices: Option<&[u32]>,
    ) {
        let resources = &self.render_context.render_resource_context.resources;
        let bind_groups = resources.bind_groups.read();
        let bind_group = bind_groups.get(&bind_group_id).unwrap();
        let buffers = resources.buffers.read();
        let textures = resources.textures.read();
        let gl = &self.render_context.device.get_context();
        for (i, binding) in bind_group.iter().enumerate() {
            let offset = dynamic_uniform_indices.map_or(0, |indices| indices[i]);
            match binding {
                crate::WebGL2RenderResourceBinding::Buffer {
                    binding_point,
                    buffer,
                    size,
                } => {
                    // log::info!(
                    //     "bind_buffer_range short_id: {:?}",
                    //     resources.short_buffer_id(*buffer)
                    // );
                    let buffer = buffers.get(&buffer).unwrap();
                    if let Buffer::WebGlBuffer(buffer_id) = &buffer.buffer {
                        gl_call!(gl.bind_buffer_range_with_i32_and_i32(
                            Gl::UNIFORM_BUFFER,
                            *binding_point,
                            Some(buffer_id),
                            offset as i32,
                            *size as i32,
                        ));
                    } else {
                        panic!("binding in-memory buffer");
                    }
                }
                crate::WebGL2RenderResourceBinding::Texture {
                    texture,
                    texture_unit,
                } => {
                    // it seems it may not work
                    // (forcing texture_unit=1 do not work properly)
                    gl_call!(gl.active_texture(Gl::TEXTURE0 + texture_unit));
                    gl_call!(gl.bind_texture(Gl::TEXTURE_2D, Some(textures.get(texture).unwrap())))
                }
                crate::WebGL2RenderResourceBinding::Sampler(_) => {
                    // TODO
                }
            }
        }
    }

    fn set_pipeline(&mut self, pipeline_handle: &Handle<PipelineDescriptor>) {
        self.pipeline = Some(pipeline_handle.as_weak());

        let resources = &self.render_context.render_resource_context.resources;
        let programs = resources.programs.read();
        let pipelines = resources.pipelines.read();
        let pipeline = pipelines.get(&pipeline_handle).unwrap();
        let program = programs.get(&pipeline.shader_stages).unwrap();

        let ctx = self.render_context;
        let gl = &ctx.device.get_context();
        gl_call!(gl.use_program(Some(&program)));
    }
}
