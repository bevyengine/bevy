use legion::world::World;
use wgpu::{CommandEncoder, Device, SwapChainOutput};

pub trait Pass {
    fn render(&mut self, device: &Device, frame: &SwapChainOutput, encoder: &mut CommandEncoder, world: &mut World);
}