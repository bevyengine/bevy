use super::{compile_shader, link_program, reflect_layout, Gl, WebGlShader};
use crate::{
    gl_call, Buffer, Device, GlBufferInfo, GlVertexBufferDescripror, WebGL2Pipeline,
    WebGL2RenderResourceBinding, WebGL2Resources,
};
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_render::{
    pipeline::{
        BindGroupDescriptor, BindGroupDescriptorId, BindType, BindingDescriptor,
        PipelineDescriptor, PipelineLayout,
    },
    renderer::{
        BindGroup, BufferId, BufferInfo, BufferUsage, RenderResourceBinding, RenderResourceContext,
        RenderResourceId, SamplerId, TextureId,
    },
    shader::{Shader, ShaderSource, ShaderStage, ShaderStages},
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_utils::HashMap;
use bevy_window::Window;
use parking_lot::RwLock;
use std::{ops::Range, sync::Arc};

#[derive(Clone)]
pub struct WebGL2RenderResourceContext {
    pub device: Arc<Device>,
    pub resources: WebGL2Resources,
    pub pipeline_descriptors: Arc<RwLock<HashMap<Handle<PipelineDescriptor>, PipelineDescriptor>>>,
    initialized: bool,
}

unsafe impl Send for WebGL2RenderResourceContext {}
unsafe impl Sync for WebGL2RenderResourceContext {}

impl WebGL2RenderResourceContext {
    pub fn new(device: Arc<crate::Device>) -> Self {
        WebGL2RenderResourceContext {
            device,
            resources: WebGL2Resources::default(),
            pipeline_descriptors: Default::default(),
            initialized: false,
        }
    }

    pub fn add_texture_descriptor(&self, texture: TextureId, descriptor: TextureDescriptor) {
        self.resources
            .texture_descriptors
            .write()
            .insert(texture, descriptor);
    }

    pub fn create_bind_group_layout(&self, descriptor: &BindGroupDescriptor) {
        if self.bind_group_descriptor_exists(descriptor.id) {
            return;
        };
        // log::info!(
        //     "resources: create bind group layoyt, descriptor: {:?}",
        //     descriptor
        // );
        self.resources
            .bind_group_layouts
            .write()
            .insert(descriptor.id, descriptor.clone());
    }

    pub fn compile_shader(&self, shader: &Shader) -> WebGlShader {
        let shader_type = match shader.stage {
            ShaderStage::Vertex => Gl::VERTEX_SHADER,
            ShaderStage::Fragment => Gl::FRAGMENT_SHADER,
            ShaderStage::Compute => panic!("compute shaders are not supported!"),
        };

        match &shader.source {
            ShaderSource::Glsl(source) => {
                compile_shader(&self.device.get_context(), shader_type, source).unwrap()
            }
            _ => {
                panic!("unsupported shader format");
            }
        }
    }

    #[allow(unused_variables)]
    pub fn initialize(&mut self, winit_window: &winit::window::Window) {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;

            let size = winit_window.inner_size();
            let gl = winit_window
                .canvas()
                .get_context("webgl2")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::WebGl2RenderingContext>()
                .unwrap();

            // let ret = gl
            //     .get_framebuffer_attachment_parameter(
            //         Gl::FRAMEBUFFER,
            //         Gl::BACK,
            //         Gl::FRAMEBUFFER_ATTACHMENT_COLOR_ENCODING,
            //     )
            //     .unwrap()
            //     .as_f64()
            //     .unwrap() as u32;

            // log::info!(
            //     "FRAMEBUFFER_ATTACHMENT_COLOR_ENCODING linear: {:?}, srgb: {:?}",
            //     ret == Gl::LINEAR,
            //     ret == Gl::SRGB
            // );

            gl_call!(gl.viewport(0, 0, size.width as i32, size.height as i32));
            gl_call!(gl.enable(Gl::BLEND));
            gl_call!(gl.enable(Gl::CULL_FACE));
            gl_call!(gl.enable(Gl::DEPTH_TEST));
            gl_call!(gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA));

            self.device.set_context(gl);
            self.initialized = true;
        }
    }
}

impl RenderResourceContext for WebGL2RenderResourceContext {
    fn flush(&self) {
        let gl = &self.device.get_context();
        gl_call!(gl.flush());
    }

    fn reflect_pipeline_layout(
        &self,
        shaders: &Assets<Shader>,
        shader_stages: &ShaderStages,
        _enforce_bevy_conventions: bool,
    ) -> PipelineLayout {
        log::info!("reflecting shader layoyut!");
        let gl_shaders: Vec<WebGlShader> = shader_stages
            .iter()
            .map(|handle| self.compile_shader(shaders.get(&handle).unwrap()))
            .collect();

        let program =
            link_program(&*self.device.get_context(), &gl_shaders).expect("WebGL program");

        log::info!("program compiled!");

        let gl = &self.device.get_context();

        let layout = reflect_layout(&*gl, &program);
        log::info!("reflected layoyt: {:#?}", layout);
        self.resources
            .programs
            .write()
            .insert(shader_stages.clone(), program);
        layout
    }

    fn render_pipeline_exists(&self, pipeline_handle: &Handle<PipelineDescriptor>) -> bool {
        self.pipeline_descriptors
            .read()
            .contains_key(&pipeline_handle)
    }

    fn get_aligned_texture_size(&self, data_size: usize) -> usize {
        data_size
    }

    fn get_aligned_uniform_size(&self, size: usize, uniform_name: Option<&str>) -> usize {
        if let Some(name) = uniform_name {
            let pipeline_descriptors = self.pipeline_descriptors.read();
            // TODO: should we iterate over all pipeline descriptors?
            // PERF: direct create name -> BindingDescriptor hashmap
            for (_, descr) in pipeline_descriptors.iter() {
                if let Some(layout) = &descr.layout {
                    let binding = layout
                        .bind_groups
                        .iter()
                        .flat_map(|c| c.bindings.iter())
                        .find(|binding| binding.name == name);
                    if let Some(BindingDescriptor {
                        bind_type: BindType::Uniform { property, .. },
                        ..
                    }) = binding
                    {
                        return size.max(16).max(property.get_size() as usize);
                    }
                }
            }
        }
        size.max(16)
    }

    fn create_swap_chain(&self, window: &Window) {
        log::info!("create_swap_chain for window: {:?}", window);
        let gl = &self.device.get_context();
        gl_call!(gl.viewport(0, 0, window.width() as i32, window.height() as i32));

    }

    fn next_swap_chain_texture(&self, _window: &Window) -> TextureId {
        //log::info!("next_swap_chain_texture");
        TextureId::new()
    }

    fn drop_swap_chain_texture(&self, _render_resource: TextureId) {
        //log::info!("drop_swap_chain_texture");
    }

    fn drop_all_swap_chain_textures(&self) {
        // log::info!("drop_all_swap_chain_textures");
    }

    fn create_sampler(&self, _sampler_descriptor: &SamplerDescriptor) -> SamplerId {
        // log::info!("create_sampler");
        SamplerId::new()
    }

    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> TextureId {
        let texture_id = TextureId::new();
        self.add_texture_descriptor(texture_id, texture_descriptor);
        let gl = &self.device.get_context();
        let texture = gl_call!(gl.create_texture()).unwrap();
        self.resources.textures.write().insert(texture_id, texture);
        texture_id
    }

    fn create_buffer(&self, info: BufferInfo) -> BufferId {
        let buffer_id = BufferId::new();
        // log::info!(
        //     "create_buffer, info: {:?}, short_id: {:?}",
        //     info,
        //     self.resources.short_buffer_id(buffer_id)
        // );

        let buffer = if info
            .buffer_usage
            .contains(BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC)
        {
            // uninitialzied in-memory buffer
            let mut data = Vec::with_capacity(info.size);
            unsafe { data.set_len(info.size) };
            Buffer::Data(data)
        } else {
            let gl = &self.device.get_context();
            let id = gl_call!(gl.create_buffer())
                .ok_or("failed to create_buffer")
                .unwrap();
            gl_call!(gl.bind_buffer(Gl::UNIFORM_BUFFER, Some(&id)));
            gl_call!(gl.buffer_data_with_i32(
                Gl::UNIFORM_BUFFER,
                info.size as i32,
                Gl::DYNAMIC_DRAW,
            ));
            Buffer::WebGlBuffer(id)
        };
        let gl_buffer_info = GlBufferInfo {
            buffer,
            info,
            vao: None,
        };
        self.resources
            .buffers
            .write()
            .insert(buffer_id, gl_buffer_info);
        buffer_id
    }

    fn write_mapped_buffer(
        &self,
        id: BufferId,
        range: Range<u64>,
        write: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) {
        // log::info!(
        //     "write_mapped_buffer, short_id: {:?}",
        //     self.resources.short_buffer_id(id)
        // );
        // TODO - for in-memory buffers find a way how to write
        // directly to buffer (it is problematic, as write callback may call
        // may call create_buffer, which locks resources.buffers)
        let size = (range.end - range.start) as usize;
        let mut data = Vec::with_capacity(size);
        unsafe { data.set_len(size) }
        write(&mut data, self);

        let mut buffers = self.resources.buffers.write();
        let buffer = buffers.get_mut(&id).unwrap();

        match &mut buffer.buffer {
            Buffer::WebGlBuffer(buffer_id) => {
                let gl = &self.device.get_context();
                gl_call!(gl.bind_buffer(Gl::COPY_WRITE_BUFFER, Some(&buffer_id)));
                gl_call!(
                    gl.buffer_sub_data_with_i32_and_u8_array_and_src_offset_and_length(
                        Gl::COPY_WRITE_BUFFER,
                        range.start as i32,
                        &data,
                        0,
                        data.len() as u32,
                    )
                );
            }
            Buffer::Data(buffer_data) => {
                let sub_data =
                    &mut buffer_data.as_mut_slice()[(range.start as usize)..(range.end as usize)];
                sub_data.copy_from_slice(&data);
            }
        }
        // log::info!(
        //     "done write_mapped_buffer, short_id: {:?}",
        //     self.resources.short_buffer_id(id)
        // );
    }

    fn map_buffer(&self, _id: BufferId) {
        // log::info!("map buffer {:?}", _id);
    }

    fn unmap_buffer(&self, _id: BufferId) {
        // log::info!("unmap buffer {:?}", _id);
    }

    fn create_buffer_with_data(&self, info: BufferInfo, data: &[u8]) -> BufferId {
        let buffer_id = BufferId::new();
        // log::info!(
        //     "create_buffer_with_data, info: {:?}, short_id: {:?}",
        //     info,
        //     self.resources.short_buffer_id(buffer_id),
        // );

        let buffer = if info
            .buffer_usage
            .contains(BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC)
        {
            // in-memory buffer
            Buffer::Data(Vec::from(data))
        } else {
            let gl = &self.device.get_context();
            let id = gl_call!(gl.create_buffer())
                .ok_or("failed to create_buffer")
                .unwrap();
            if info.buffer_usage & BufferUsage::VERTEX == BufferUsage::VERTEX {
                gl_call!(gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&id)));
                gl_call!(gl.buffer_data_with_u8_array(Gl::ARRAY_BUFFER, &data, Gl::DYNAMIC_DRAW));
            } else if info.buffer_usage & BufferUsage::INDEX == BufferUsage::INDEX {
                gl_call!(gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&id)));
                gl_call!(gl.buffer_data_with_u8_array(
                    Gl::ELEMENT_ARRAY_BUFFER,
                    &data,
                    Gl::DYNAMIC_DRAW
                ));
            } else {
                gl_call!(gl.bind_buffer(Gl::PIXEL_UNPACK_BUFFER, Some(&id)));
                gl_call!(gl.buffer_data_with_u8_array(
                    Gl::PIXEL_UNPACK_BUFFER,
                    &data,
                    Gl::DYNAMIC_DRAW
                ));
            };
            Buffer::WebGlBuffer(id)
        };

        let gl_buffer_info = GlBufferInfo {
            buffer,
            info,
            vao: None,
        };
        self.resources
            .buffers
            .write()
            .insert(buffer_id, gl_buffer_info);
        buffer_id
    }

    fn create_shader_module(&self, _shader_handle: &Handle<Shader>, _shaders: &Assets<Shader>) {}

    fn remove_buffer(&self, buffer: BufferId) {
        let gl = &self.device.get_context();
        let mut buffers = self.resources.buffers.write();
        let gl_buffer = buffers.remove(&buffer).unwrap();
        if let Some(vao) = gl_buffer.vao {
            gl_call!(gl.delete_vertex_array(Some(&vao)));
        }
        if let Buffer::WebGlBuffer(buffer_id) = &gl_buffer.buffer {
            gl_call!(gl.delete_buffer(Some(buffer_id)));
        }
    }

    fn remove_texture(&self, texture: TextureId) {
        let gl = &self.device.get_context();
        let mut texture_descriptors = self.resources.texture_descriptors.write();
        let mut textures = self.resources.textures.write();
        let gl_texture = textures.remove(&texture).unwrap();
        gl_call!(gl.delete_texture(Some(&gl_texture)));
        texture_descriptors.remove(&texture);
    }

    fn remove_sampler(&self, _sampler: SamplerId) {}

    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        render_resource: RenderResourceId,
        index: u64,
    ) {
        self.resources
            .asset_resources
            .write()
            .insert((handle, index), render_resource);
    }

    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: u64,
    ) -> Option<RenderResourceId> {
        self.resources
            .asset_resources
            .write()
            .get(&(handle, index))
            .cloned()
    }

    fn create_render_pipeline(
        &self,
        source_pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
        _shaders: &Assets<Shader>,
    ) {
        // log::info!(
        //     "create render pipeline: source_handle: {:?} handle: {:?}, descriptor: {:#?}",
        //     source_pipeline_handle,
        //     pipeline_handle,
        //     pipeline_descriptor
        // );
        let layout = pipeline_descriptor.get_layout().unwrap();
        self.pipeline_descriptors
            .write()
            .insert(source_pipeline_handle, pipeline_descriptor.clone());
        for bind_group_descriptor in layout.bind_groups.iter() {
            self.create_bind_group_layout(&bind_group_descriptor);
        }
        let vertex_buffer_descriptors = pipeline_descriptor
            .layout
            .as_ref()
            .unwrap()
            .vertex_buffer_descriptors
            .clone();
        let gl = &self.device.get_context();

        let programs = self.resources.programs.read();
        let program = programs.get(&pipeline_descriptor.shader_stages).unwrap();
        log::info!("found compiled program: {:?}", program);
        gl_call!(gl.use_program(Some(&program)));
        log::info!("start binding");
        for bind_group in layout.bind_groups.iter() {
            for binding in bind_group.bindings.iter() {
                let block_index = gl_call!(gl.get_uniform_block_index(&program, &binding.name));
                log::info!("trying to bind {:?}", binding.name);
                if (block_index as i32) < 0 {
                    log::info!("invalid block index for {:?}, skipping", &binding.name);
                    if let Some(uniform_location) =
                        gl_call!(gl.get_uniform_location(&program, &binding.name))
                    {
                        log::info!("found uniform location: {:?}", uniform_location);
                        if let BindType::SampledTexture { .. } = binding.bind_type {
                            let texture_unit = self
                                .resources
                                .get_or_create_texture_unit(bind_group.index, binding.index);
                            gl_call!(gl.uniform1i(Some(&uniform_location), texture_unit as i32));
                            log::info!(
                                "found texture uniform {:?}, binding to unit {:?}",
                                binding.name,
                                texture_unit
                            );
                        } else {
                            panic!("use non-block uniforms expected only for textures");
                        }
                    } else {
                        log::info!("can't bind {:?}", binding.name);
                    }
                    continue;
                }
                let binding_point = self
                    .resources
                    .get_or_create_binding_point(bind_group.index, binding.index);
                gl_call!(gl.uniform_block_binding(&program, block_index, binding_point));
                let _min_data_size = gl_call!(gl.get_active_uniform_block_parameter(
                    &program,
                    block_index,
                    Gl::UNIFORM_BLOCK_DATA_SIZE,
                ))
                .unwrap();
                log::info!(
                    "uniform_block_binding: name: {:?}, block_index: {:?}, binding_point: {:?}, min_data_size: {:?}",
                    binding.name,
                    block_index,
                    binding_point,
                    _min_data_size,
                );
            }
        }
        log::info!("done binding");

        let vertex_buffer_descriptors = vertex_buffer_descriptors
            .iter()
            .map(|vertex_buffer_descriptor| {
                GlVertexBufferDescripror::from(gl, program, vertex_buffer_descriptor)
            })
            .collect();

        let pipeline = WebGL2Pipeline {
            shader_stages: pipeline_descriptor.shader_stages.clone(),
            vertex_buffer_descriptors,
        };
        self.resources
            .pipelines
            .write()
            .insert(pipeline_handle, pipeline);
    }

    fn create_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: &BindGroup,
    ) {
        assert!(self.bind_group_descriptor_exists(bind_group_descriptor_id));
        let layouts = self.resources.bind_group_layouts.read();
        let bind_group_layout = layouts.get(&bind_group_descriptor_id).unwrap();
        let _gl = &self.device.get_context();
        let mut bind_groups = self.resources.bind_groups.write();
        if bind_groups.get(&bind_group.id).is_some() {
            return;
        }
        let bind_group_vec: Vec<_> = bind_group
            .indexed_bindings
            .iter()
            .filter(|entry| {
                entry.entry.get_buffer().is_some() || entry.entry.get_texture().is_some()
            }) // TODO
            .map(|entry| match &entry.entry {
                RenderResourceBinding::Buffer { buffer, range, .. } => {
                    let binding_point = self
                        .resources
                        .get_or_create_binding_point(bind_group_layout.index, entry.index);
                    WebGL2RenderResourceBinding::Buffer {
                        binding_point,
                        buffer: *buffer,
                        size: range.end - range.start,
                    }
                }
                RenderResourceBinding::Texture(texture) => {
                    let texture_unit = self
                        .resources
                        .get_or_create_texture_unit(bind_group_layout.index, entry.index);
                    WebGL2RenderResourceBinding::Texture {
                        texture: *texture,
                        texture_unit,
                    }
                }
                RenderResourceBinding::Sampler(sampler) => {
                    WebGL2RenderResourceBinding::Sampler(*sampler)
                }
            })
            .collect();
        bind_groups.insert(bind_group.id, bind_group_vec);
    }

    fn create_shader_module_from_source(&self, _shader_handle: &Handle<Shader>, _shader: &Shader) {}

    fn remove_asset_resource_untyped(&self, handle: HandleUntyped, index: u64) {
        self.resources
            .asset_resources
            .write()
            .remove(&(handle, index));
    }

    fn clear_bind_groups(&self) {
        self.resources.bind_groups.write().clear();
    }

    fn get_buffer_info(&self, buffer: BufferId) -> Option<BufferInfo> {
        self.resources
            .buffers
            .read()
            .get(&buffer)
            .map(|f| f.info.clone())
    }

    fn bind_group_descriptor_exists(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
    ) -> bool {
        return self
            .resources
            .bind_group_layouts
            .read()
            .contains_key(&bind_group_descriptor_id);
    }
}
