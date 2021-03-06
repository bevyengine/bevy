mod anchors;
pub mod entity;
mod flex;
mod focus;
mod margins;
mod render;
mod ui_node;
pub mod update;
pub mod widget;

pub use anchors::*;
use bevy_math::{Rect, Size};
use bevy_render::RenderStage;
pub use flex::*;
pub use focus::*;
pub use margins::*;
pub use render::*;
pub use ui_node::*;

pub mod prelude {
    pub use crate::{entity::*, ui_node::*, widget::Button, Anchors, Interaction, Margins};
}

use bevy_app::prelude::*;
use bevy_ecs::{
    schedule::{ParallelSystemDescriptorCoercion, StageLabel, SystemLabel, SystemStage},
    system::IntoSystem,
};
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
            .register_type::<AlignContent>()
            .register_type::<AlignItems>()
            .register_type::<AlignSelf>()
            .register_type::<Direction>()
            .register_type::<Display>()
            .register_type::<FlexDirection>()
            .register_type::<FlexWrap>()
            .register_type::<JustifyContent>()
            .register_type::<Node>()
            .register_type::<PositionType>()
            .register_type::<Size<f32>>()
            .register_type::<Size<Val>>()
            .register_type::<Rect<Val>>()
            .register_type::<Style>()
            .register_type::<Val>()
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

        crate::render::add_ui_graph(app.world_mut());
    }
}
