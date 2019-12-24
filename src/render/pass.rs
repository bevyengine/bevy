use legion::world::World;
use wgpu::{Buffer, CommandEncoder, Device, SwapChainDescriptor, SwapChainOutput};
use crate::render::RenderResources;

pub trait Pass {
    fn render(&mut self, device: &Device, frame: &SwapChainOutput, encoder: &mut CommandEncoder, world: &mut World, render_resources: &RenderResources);
    fn resize(&mut self, device: &Device, frame: &SwapChainDescriptor);
    fn get_camera_uniform_buffer(&self) -> Option<&Buffer>;
}