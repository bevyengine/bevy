use bevy_render2::{pipeline::BindGroupDescriptorId, render_resource::BindGroupId};
use bevy_utils::tracing::trace;
use crate::resources::WgpuResourceRefs;

pub enum Pass<'a, 'b> {
    Render(&'b mut wgpu::RenderPass<'a>),
    Compute(&'b mut wgpu::ComputePass<'a>),
}

pub fn set_bind_group<'a, 'b>(
    pass: Pass<'a, 'b>,
    wgpu_resources: &WgpuResourceRefs<'a>,
    index: u32,
    bind_group_descriptor_id: BindGroupDescriptorId,
    bind_group: BindGroupId,
    dynamic_uniform_indices: Option<&[u32]>,
) {
    if let Some(bind_group_info) = wgpu_resources
        .bind_groups
        .get(&bind_group_descriptor_id)
    {
        if let Some(wgpu_bind_group) = bind_group_info.bind_groups.get(&bind_group) {
            const EMPTY: &[u32] = &[];
            let dynamic_uniform_indices =
                if let Some(dynamic_uniform_indices) = dynamic_uniform_indices {
                    dynamic_uniform_indices
                } else {
                    EMPTY
                };
            wgpu_resources
                .used_bind_group_sender
                .send(bind_group)
                .unwrap();

            trace!(
                "set bind group {:?} {:?}: {:?}",
                bind_group_descriptor_id,
                dynamic_uniform_indices,
                bind_group
            );
            
            match pass {
                Pass::Render(render_pass) =>
                    render_pass.set_bind_group(index, wgpu_bind_group, dynamic_uniform_indices),
                Pass::Compute(compute_pass) => 
                    compute_pass.set_bind_group(index, wgpu_bind_group, dynamic_uniform_indices),
            }
        }
    }
}