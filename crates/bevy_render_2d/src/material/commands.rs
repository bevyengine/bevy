use core::marker::PhantomData;

use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::{
        PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
    },
};

use crate::mesh_pipeline::commands::{DrawMesh2d, SetMesh2dBindGroup, SetMesh2dViewBindGroup};

use super::{instances::RenderMaterial2dInstances, prepared_asset::PreparedMaterial2d, Material2d};

pub type DrawMaterial2d<M> = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    SetMaterial2dBindGroup<M, 2>,
    DrawMesh2d,
);

pub struct SetMaterial2dBindGroup<M: Material2d, const I: usize>(PhantomData<M>);

impl<P: PhaseItem, M: Material2d, const I: usize> RenderCommand<P>
    for SetMaterial2dBindGroup<M, I>
{
    type Param = (
        SRes<RenderAssets<PreparedMaterial2d<M>>>,
        SRes<RenderMaterial2dInstances<M>>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (materials, material_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let materials = materials.into_inner();
        let material_instances = material_instances.into_inner();
        let Some(material_instance) = material_instances.get(&item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let Some(material2d) = materials.get(*material_instance) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &material2d.bind_group, &[]);
        RenderCommandResult::Success
    }
}
