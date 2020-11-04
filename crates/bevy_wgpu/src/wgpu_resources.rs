use bevy_asset::{Handle, HandleUntyped};
use bevy_render::{
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    renderer::{BindGroupId, BufferId, BufferInfo, RenderResourceId, SamplerId, TextureId},
    shader::Shader,
    texture::TextureDescriptor,
};
use bevy_utils::AhashMap;
use bevy_window::WindowId;
use parking_lot::{RwLock, RwLockReadGuard};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct WgpuBindGroupInfo {
    pub bind_groups: AhashMap<BindGroupId, wgpu::BindGroup>,
}

/// Grabs a read lock on all wgpu resources. When paired with WgpuResourceRefs, this allows
/// you to pass in wgpu resources to wgpu::RenderPass<'a> with the appropriate lifetime. This is accomplished by
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
#[derive(Debug)]
pub struct WgpuResourcesReadLock<'a> {
    pub buffers: RwLockReadGuard<'a, AhashMap<BufferId, Arc<wgpu::Buffer>>>,
    pub textures: RwLockReadGuard<'a, AhashMap<TextureId, wgpu::TextureView>>,
    pub swap_chain_frames: RwLockReadGuard<'a, AhashMap<TextureId, wgpu::SwapChainFrame>>,
    pub render_pipelines:
        RwLockReadGuard<'a, AhashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>>,
    pub bind_groups: RwLockReadGuard<'a, AhashMap<BindGroupDescriptorId, WgpuBindGroupInfo>>,
}

impl<'a> WgpuResourcesReadLock<'a> {
    pub fn refs(&'a self) -> WgpuResourceRefs<'a> {
        WgpuResourceRefs {
            buffers: &self.buffers,
            textures: &self.textures,
            swap_chain_frames: &self.swap_chain_frames,
            render_pipelines: &self.render_pipelines,
            bind_groups: &self.bind_groups,
        }
    }
}

/// Stores read only references to WgpuResource collections. See WgpuResourcesReadLock docs for context on why this exists
#[derive(Debug)]
pub struct WgpuResourceRefs<'a> {
    pub buffers: &'a AhashMap<BufferId, Arc<wgpu::Buffer>>,
    pub textures: &'a AhashMap<TextureId, wgpu::TextureView>,
    pub swap_chain_frames: &'a AhashMap<TextureId, wgpu::SwapChainFrame>,
    pub render_pipelines: &'a AhashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>,
    pub bind_groups: &'a AhashMap<BindGroupDescriptorId, WgpuBindGroupInfo>,
}

#[derive(Default, Clone, Debug)]
pub struct WgpuResources {
    pub buffer_infos: Arc<RwLock<AhashMap<BufferId, BufferInfo>>>,
    pub texture_descriptors: Arc<RwLock<AhashMap<TextureId, TextureDescriptor>>>,
    pub window_surfaces: Arc<RwLock<AhashMap<WindowId, wgpu::Surface>>>,
    pub window_swap_chains: Arc<RwLock<AhashMap<WindowId, wgpu::SwapChain>>>,
    pub swap_chain_frames: Arc<RwLock<AhashMap<TextureId, wgpu::SwapChainFrame>>>,
    pub buffers: Arc<RwLock<AhashMap<BufferId, Arc<wgpu::Buffer>>>>,
    pub texture_views: Arc<RwLock<AhashMap<TextureId, wgpu::TextureView>>>,
    pub textures: Arc<RwLock<AhashMap<TextureId, wgpu::Texture>>>,
    pub samplers: Arc<RwLock<AhashMap<SamplerId, wgpu::Sampler>>>,
    pub shader_modules: Arc<RwLock<AhashMap<Handle<Shader>, wgpu::ShaderModule>>>,
    pub render_pipelines: Arc<RwLock<AhashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>>>,
    pub bind_groups: Arc<RwLock<AhashMap<BindGroupDescriptorId, WgpuBindGroupInfo>>>,
    pub bind_group_layouts: Arc<RwLock<AhashMap<BindGroupDescriptorId, wgpu::BindGroupLayout>>>,
    pub asset_resources: Arc<RwLock<AhashMap<(HandleUntyped, u64), RenderResourceId>>>,
}

impl WgpuResources {
    pub fn read(&self) -> WgpuResourcesReadLock {
        WgpuResourcesReadLock {
            buffers: self.buffers.read(),
            textures: self.texture_views.read(),
            swap_chain_frames: self.swap_chain_frames.read(),
            render_pipelines: self.render_pipelines.read(),
            bind_groups: self.bind_groups.read(),
        }
    }

    pub fn has_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group_id: BindGroupId,
    ) -> bool {
        if let Some(bind_group_info) = self.bind_groups.read().get(&bind_group_descriptor_id) {
            bind_group_info.bind_groups.get(&bind_group_id).is_some()
        } else {
            false
        }
    }
}
