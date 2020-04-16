use crate::{renderer_2::WgpuRenderResourceContext, wgpu_type_converter::WgpuInto};
use bevy_asset::{Handle, HandleUntyped};
use bevy_render::{
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    render_resource::{BufferInfo, RenderResource, RenderResourceSetId, ResourceInfo},
    renderer_2::RenderResourceContext,
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_window::{Window, WindowId};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard},
};

#[derive(Default)]
pub struct WgpuBindGroupInfo {
    pub bind_groups: HashMap<RenderResourceSetId, wgpu::BindGroup>,
}

/// Grabs a read lock on all wgpu resources. When paired with WgpuResourceRefs, this allows
/// us to pass in wgpu resources to wgpu::RenderPass<'a> with the appropriate lifetime. This is accomplished by
/// grabbing a WgpuResourcesReadLock _before_ creating a wgpu::RenderPass, getting a WgpuResourcesRefs, and storing that
/// in the pass.
///
/// This is only a problem because RwLockReadGuard.read() erases the guard's lifetime and creates a new anonymous lifetime. If
/// you call RwLockReadGuard.read() during a pass, the reference will have an anonymous lifetime that lives for less than the
/// pass, which violates the lifetime constraints in place.
///
/// The biggest implication of this design (other than the additional boilerplate here) is that beginning a render pass
/// blocks writes to these resources. This means that if the pass attempts to write any resource, a deadlock will occur. WgpuResourceRefs
/// only has immutable references, so the only way to make a deadlock happen is to access WgpuResources directly in the pass. It also means
/// that other threads attempting to write resources will need to wait for pass encoding to finish. Almost all writes should occur before
/// passes start, so this hopefully won't be a problem.
///
/// It is worth comparing the performance of this to transactional / copy-based approaches. This lock based design guarantees
/// consistency, doesn't perform redundant allocations, and only blocks when a write is occurring. A copy based approach would
/// never block, but would require more allocations / state-synchronization, which I expect will be more expensive. It would also be
/// "eventually consistent" instead of "strongly consistent".
///
/// Single threaded implementations don't need to worry about these lifetimes constraints at all. RenderPasses can use a RenderContext's
/// WgpuResources directly. RenderContext already has a lifetime greater than the RenderPass.
pub struct WgpuResourcesReadLock<'a> {
    pub buffers: RwLockReadGuard<'a, HashMap<RenderResource, wgpu::Buffer>>,
    pub textures: RwLockReadGuard<'a, HashMap<RenderResource, wgpu::TextureView>>,
    pub swap_chain_outputs: RwLockReadGuard<'a, HashMap<WindowId, wgpu::SwapChainOutput>>,
    pub render_pipelines:
        RwLockReadGuard<'a, HashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>>,
    pub bind_groups: RwLockReadGuard<'a, HashMap<BindGroupDescriptorId, WgpuBindGroupInfo>>,
}

impl<'a> WgpuResourcesReadLock<'a> {
    pub fn refs(&'a self) -> WgpuResourceRefs<'a> {
        WgpuResourceRefs {
            buffers: &self.buffers,
            textures: &self.textures,
            swap_chain_outputs: &self.swap_chain_outputs,
            render_pipelines: &self.render_pipelines,
            bind_groups: &self.bind_groups,
        }
    }
}

/// Stores read only references to WgpuResource collections. See WgpuResourcesReadLock docs for context on why this exists
pub struct WgpuResourceRefs<'a> {
    pub buffers: &'a HashMap<RenderResource, wgpu::Buffer>,
    pub textures: &'a HashMap<RenderResource, wgpu::TextureView>,
    pub swap_chain_outputs: &'a HashMap<WindowId, wgpu::SwapChainOutput>,
    pub render_pipelines: &'a HashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>,
    pub bind_groups: &'a HashMap<BindGroupDescriptorId, WgpuBindGroupInfo>,
}

#[derive(Default, Clone)]
pub struct WgpuResources {
    // TODO: remove this from WgpuResources. it doesn't need to be here
    pub window_surfaces: Arc<RwLock<HashMap<WindowId, wgpu::Surface>>>,
    pub window_swap_chains: Arc<RwLock<HashMap<WindowId, wgpu::SwapChain>>>,
    pub swap_chain_outputs: Arc<RwLock<HashMap<WindowId, wgpu::SwapChainOutput>>>,
    pub buffers: Arc<RwLock<HashMap<RenderResource, wgpu::Buffer>>>,
    pub textures: Arc<RwLock<HashMap<RenderResource, wgpu::TextureView>>>,
    pub samplers: Arc<RwLock<HashMap<RenderResource, wgpu::Sampler>>>,
    pub resource_info: Arc<RwLock<HashMap<RenderResource, ResourceInfo>>>,
    pub shader_modules: Arc<RwLock<HashMap<Handle<Shader>, wgpu::ShaderModule>>>,
    pub render_pipelines: Arc<RwLock<HashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>>>,
    pub bind_groups: Arc<RwLock<HashMap<BindGroupDescriptorId, WgpuBindGroupInfo>>>,
    pub bind_group_layouts: Arc<RwLock<HashMap<BindGroupDescriptorId, wgpu::BindGroupLayout>>>,
    pub asset_resources: Arc<RwLock<HashMap<(HandleUntyped, usize), RenderResource>>>,
}

impl WgpuResources {
    pub fn read<'a>(&'a self) -> WgpuResourcesReadLock<'a> {
        WgpuResourcesReadLock {
            buffers: self.buffers.read().unwrap(),
            textures: self.textures.read().unwrap(),
            swap_chain_outputs: self.swap_chain_outputs.read().unwrap(),
            render_pipelines: self.render_pipelines.read().unwrap(),
            bind_groups: self.bind_groups.read().unwrap(),
        }
    }

    pub fn set_window_surface(&self, window_id: WindowId, surface: wgpu::Surface) {
        self.window_surfaces
            .write()
            .unwrap()
            .insert(window_id, surface);
    }

    pub fn next_swap_chain_texture(&self, window_id: WindowId) {
        let mut swap_chain_outputs = self.window_swap_chains.write().unwrap();
        let swap_chain_output = swap_chain_outputs.get_mut(&window_id).unwrap();
        let next_texture = swap_chain_output.get_next_texture().unwrap();
        self.swap_chain_outputs
            .write()
            .unwrap()
            .insert(window_id, next_texture);
    }

    pub fn remove_swap_chain_texture(&self, window_id: WindowId) {
        self.swap_chain_outputs.write().unwrap().remove(&window_id);
    }

    pub fn remove_all_swap_chain_textures(&self) {
        self.swap_chain_outputs.write().unwrap().clear();
    }

    pub fn create_window_swap_chain(&self, device: &wgpu::Device, window: &Window) {
        let swap_chain_descriptor: wgpu::SwapChainDescriptor = window.wgpu_into();
        let surfaces = self.window_surfaces.read().unwrap();
        let surface = surfaces
            .get(&window.id)
            .expect("No surface found for window");
        let swap_chain = device.create_swap_chain(surface, &swap_chain_descriptor);
        self.window_swap_chains
            .write()
            .unwrap()
            .insert(window.id, swap_chain);
    }

    pub fn add_resource_info(&self, resource: RenderResource, resource_info: ResourceInfo) {
        self.resource_info
            .write()
            .unwrap()
            .insert(resource, resource_info);
    }

    pub fn has_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
    ) -> bool {
        if let Some(bind_group_info) = self
            .bind_groups
            .read()
            .unwrap()
            .get(&bind_group_descriptor_id)
        {
            bind_group_info
                .bind_groups
                .get(&render_resource_set_id)
                .is_some()
        } else {
            false
        }
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        render_resource_set_id: RenderResourceSetId,
        bind_group_descriptor: &wgpu::BindGroupDescriptor,
    ) -> wgpu::BindGroup {
        log::trace!(
            "created bind group for RenderResourceSet {:?}",
            render_resource_set_id
        );
        log::trace!("{:#?}", bind_group_descriptor);
        device.create_bind_group(bind_group_descriptor)
    }

    pub fn set_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
        bind_group: wgpu::BindGroup,
    ) {
        let mut bind_groups = self.bind_groups.write().unwrap();
        let bind_group_info = bind_groups
            .entry(bind_group_descriptor_id)
            .or_insert_with(|| WgpuBindGroupInfo::default());
        bind_group_info
            .bind_groups
            .insert(render_resource_set_id, bind_group);
    }

    pub fn create_buffer(&self, device: &wgpu::Device, buffer_info: BufferInfo) -> RenderResource {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
        });

        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Buffer(buffer_info));

        self.buffers.write().unwrap().insert(resource, buffer);
        resource
    }

    pub fn create_buffer_with_data(
        &self,
        device: &wgpu::Device,
        mut buffer_info: BufferInfo,
        data: &[u8],
    ) -> RenderResource {
        buffer_info.size = data.len();
        let buffer = device.create_buffer_with_data(data, buffer_info.buffer_usage.wgpu_into());
        self.assign_buffer(buffer, buffer_info)
    }

    // TODO: taking a closure isn't fantastic. is there any way to make this better without exposing the lock in the interface?
    pub fn get_resource_info(
        &self,
        resource: RenderResource,
        handle_info: &mut dyn FnMut(Option<&ResourceInfo>),
    ) {
        let resource_info = self.resource_info.read().unwrap();
        let info = resource_info.get(&resource);
        handle_info(info);
    }

    pub fn remove_buffer(&self, resource: RenderResource) {
        self.buffers.write().unwrap().remove(&resource);
        self.resource_info.write().unwrap().remove(&resource);
    }

    pub fn assign_buffer(&self, buffer: wgpu::Buffer, buffer_info: BufferInfo) -> RenderResource {
        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Buffer(buffer_info));
        self.buffers.write().unwrap().insert(resource, buffer);
        resource
    }

    // TODO: clean this up
    pub fn begin_create_buffer_mapped_render_context(
        buffer_info: &BufferInfo,
        render_resources: &WgpuRenderResourceContext,
        setup_data: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) -> wgpu::Buffer {
        let device = render_resources.device.clone();
        let mut mapped = device.create_buffer_mapped(&wgpu::BufferDescriptor {
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
            label: None,
        });
        setup_data(&mut mapped.data, render_resources);
        mapped.finish()
    }

    pub fn copy_buffer_to_buffer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        let buffers = self.buffers.read().unwrap();
        let source = buffers.get(&source_buffer).unwrap();
        let destination = buffers.get(&destination_buffer).unwrap();
        encoder.copy_buffer_to_buffer(source, source_offset, destination, destination_offset, size);
    }

    pub fn create_shader_module(
        &self,
        device: &wgpu::Device,
        shader_handle: Handle<Shader>,
        shader: &Shader,
    ) {
        let shader_module = device.create_shader_module(&shader.get_spirv(None));
        self.shader_modules
            .write()
            .unwrap()
            .insert(shader_handle, shader_module);
    }

    pub fn create_sampler(
        &self,
        device: &wgpu::Device,
        sampler_descriptor: &SamplerDescriptor,
    ) -> RenderResource {
        let descriptor: wgpu::SamplerDescriptor = (*sampler_descriptor).wgpu_into();
        let sampler = device.create_sampler(&descriptor);
        let resource = RenderResource::new();
        self.samplers.write().unwrap().insert(resource, sampler);
        self.add_resource_info(resource, ResourceInfo::Sampler);
        resource
    }

    pub fn create_texture(
        &self,
        device: &wgpu::Device,
        texture_descriptor: &TextureDescriptor,
    ) -> RenderResource {
        let descriptor: wgpu::TextureDescriptor = (*texture_descriptor).wgpu_into();
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_default_view();
        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Texture);
        self.textures
            .write()
            .unwrap()
            .insert(resource, texture_view);
        resource
    }

    pub fn create_texture_with_data(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        let descriptor: wgpu::TextureDescriptor = (*texture_descriptor).wgpu_into();
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_default_view();
        let temp_buf = device.create_buffer_with_data(bytes, wgpu::BufferUsage::COPY_SRC);
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                bytes_per_row: 4 * descriptor.size.width,
                rows_per_image: 0, // NOTE: Example sets this to 0, but should it be height?
            },
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
            },
            descriptor.size,
        );

        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Texture);
        self.textures
            .write()
            .unwrap()
            .insert(resource, texture_view);
        resource
    }

    pub fn remove_texture(&self, resource: RenderResource) {
        self.textures.write().unwrap().remove(&resource);
        self.resource_info.write().unwrap().remove(&resource);
    }

    pub fn remove_sampler(&self, resource: RenderResource) {
        self.samplers.write().unwrap().remove(&resource);
        self.resource_info.write().unwrap().remove(&resource);
    }

    pub fn set_asset_resource<T>(
        &mut self,
        handle: Handle<T>,
        render_resource: RenderResource,
        index: usize,
    ) where
        T: 'static,
    {
        self.asset_resources
            .write()
            .unwrap()
            .insert((handle.into(), index), render_resource);
    }

    pub fn get_asset_resource<T>(
        &mut self,
        handle: Handle<T>,
        index: usize,
    ) -> Option<RenderResource>
    where
        T: 'static,
    {
        self.asset_resources
            .write()
            .unwrap()
            .get(&(handle.into(), index))
            .cloned()
    }

    pub fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        render_resource: RenderResource,
        index: usize,
    ) {
        self.asset_resources
            .write()
            .unwrap()
            .insert((handle, index), render_resource);
    }

    pub fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: usize,
    ) -> Option<RenderResource> {
        self.asset_resources
            .write()
            .unwrap()
            .get(&(handle, index))
            .cloned()
    }

    pub fn create_bind_group_layout(
        &self,
        device: &wgpu::Device,
        bind_group_id: BindGroupDescriptorId,
        descriptor: &wgpu::BindGroupLayoutDescriptor,
    ) {
        let wgpu_bind_group_layout = device.create_bind_group_layout(descriptor);
        self.bind_group_layouts
            .write()
            .unwrap()
            .insert(bind_group_id, wgpu_bind_group_layout);
    }

    pub fn create_render_pipeline(
        &self,
        device: &wgpu::Device,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&descriptor)
    }

    pub fn set_render_pipeline(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
        render_pipeline: wgpu::RenderPipeline,
    ) {
        self.render_pipelines
            .write()
            .unwrap()
            .insert(pipeline_handle, render_pipeline);
    }
}
