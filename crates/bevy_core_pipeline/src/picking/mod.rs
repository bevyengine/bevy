use bevy_app::{CoreStage, Plugin};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    picking::{self, Picking},
    RenderApp, RenderStage,
};

pub mod node;

/// Uses the GPU to provide a buffer which allows lookup of entities at a given coordinate.
#[derive(Default)]
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Return early if no render app, this can happen in headless situations.
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app.add_system_to_stage(RenderStage::Prepare, picking::prepare_picking_targets);

        app.add_plugin(ExtractComponentPlugin::<Picking>::default())
            .add_system_to_stage(CoreStage::PreUpdate, picking::map_buffers)
            .add_system_to_stage(CoreStage::PostUpdate, picking::unmap_buffers);
    }
}
