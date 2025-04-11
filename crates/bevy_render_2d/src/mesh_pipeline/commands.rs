use bevy_ecs::{
    query::ROQueryItem,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_render::{
    mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
    render_asset::RenderAssets,
    render_phase::{
        PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult, TrackedRenderPass,
    },
    view::ViewUniformOffset,
};

use super::render::{
    Mesh2dBindGroup, Mesh2dViewBindGroup, RenderMesh2dInstance, RenderMesh2dInstances,
};

pub struct SetMesh2dViewBindGroup<const I: usize>;

pub struct SetMesh2dBindGroup<const I: usize>;

pub struct DrawMesh2d;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMesh2dViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<ViewUniformOffset>, Read<Mesh2dViewBindGroup>);
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, mesh2d_view_bind_group): ROQueryItem<'w, Self::ViewQuery>,
        _view: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &mesh2d_view_bind_group.value, &[view_uniform.offset]);

        RenderCommandResult::Success
    }
}

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMesh2dBindGroup<I> {
    type Param = SRes<Mesh2dBindGroup>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        mesh2d_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mut dynamic_offsets: [u32; 1] = Default::default();
        let mut offset_count = 0;
        if let PhaseItemExtraIndex::DynamicOffset(dynamic_offset) = item.extra_index() {
            dynamic_offsets[offset_count] = dynamic_offset;
            offset_count += 1;
        }
        pass.set_bind_group(
            I,
            &mesh2d_bind_group.into_inner().value,
            &dynamic_offsets[..offset_count],
        );
        RenderCommandResult::Success
    }
}

impl<P: PhaseItem> RenderCommand<P> for DrawMesh2d {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMesh2dInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (meshes, render_mesh2d_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
            render_mesh2d_instances.get(&item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.get(*mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

        let batch_range = item.batch_range();
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);

                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    batch_range.clone(),
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, batch_range.clone());
            }
        }
        RenderCommandResult::Success
    }
}
