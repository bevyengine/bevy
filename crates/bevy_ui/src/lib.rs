mod anchors;
pub mod entity;
mod flex;
mod focus;
mod margins;
mod node;
mod render;
pub mod update;
pub mod widget;

pub use anchors::*;
pub use flex::*;
pub use focus::*;
pub use margins::*;
pub use node::*;
pub use render::*;

pub mod prelude {
    pub use crate::{entity::*, node::*, widget::Button, Anchors, Interaction, Margins};
}

use bevy_app::prelude::*;
use bevy_ecs::{
    IntoSystem, ParallelSystemDescriptorCoercion, StageLabel, SystemLabel, SystemStage,
};
use bevy_render::{render_graph::RenderGraph, RenderStage};
use update::ui_z_system;

#[derive(Default)]
pub struct UiPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum UiStage {
    Ui,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum UiSystem {
    Flex,
}

pub mod system {
    pub const FLEX: &str = "flex";
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<FlexSurface>()
            .add_stage_before(CoreStage::PostUpdate, UiStage::Ui, SystemStage::parallel())
            .add_system_to_stage(CoreStage::PreUpdate, ui_focus_system.system())
            // add these stages to front because these must run before transform update systems
            .add_system_to_stage(
                UiStage::Ui,
                widget::text_system.system().before(UiSystem::Flex),
            )
            .add_system_to_stage(
                UiStage::Ui,
                widget::image_node_system.system().before(UiSystem::Flex),
            )
            .add_system_to_stage(UiStage::Ui, flex_node_system.system().label(UiSystem::Flex))
            .add_system_to_stage(UiStage::Ui, ui_z_system.system())
            .add_system_to_stage(RenderStage::Draw, widget::draw_text_system.system());

        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_ui_graph(resources);
    }
}
