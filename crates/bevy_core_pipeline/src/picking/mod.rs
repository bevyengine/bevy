use bevy_app::{CoreStage, Plugin};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    picking::{self, PickedEvent, Picking},
    RenderApp, RenderStage,
};

pub mod node;

/// Uses the GPU to provide a buffer which allows lookup of entities at a given coordinate.
#[derive(Default)]
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // TODO: Also register type?
        app.add_event::<PickedEvent>()
            .add_plugin(ExtractComponentPlugin::<Picking>::default())
            // In PreUpdate such that written events are ensure not to have a frame delay
            // for default user systems
            .add_system_to_stage(CoreStage::PreUpdate, picking::picking_events);

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app.add_system_to_stage(RenderStage::Prepare, picking::prepare_picking_targets);
    }
}
