mod anchors;
mod flex;
mod focus;
mod margins;
mod render;
mod ui_node;

pub mod entity;
pub mod update;
pub mod widget;

pub use anchors::*;
pub use flex::*;
pub use focus::*;
pub use margins::*;
pub use render::*;
pub use ui_node::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{entity::*, ui_node::*, widget::Button, Anchors, Interaction, Margins};
}

use bevy_app::prelude::*;
use bevy_ecs::{prelude::{ScheduleConfig, StageConfig}, schedule::SystemLabel};
use bevy_input::InputSystem;
use bevy_math::{Rect, Size};
use bevy_render::RenderStage;
use bevy_transform::TransformSystem;
use update::ui_z_system;

#[derive(Default)]
pub struct UiPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum UiSystem {
    /// After this label, the ui flex state has been updated
    Flex,
    Focus,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
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
            .add_system(
                ui_focus_system
                    .stage(CoreStage::PreUpdate)
                    .label(UiSystem::Focus)
                    .after(InputSystem),
            )
            // add these stages to front because these must run before transform update systems
            .add_system(
                widget::text_system
                    .stage(CoreStage::PostUpdate)
                    .before(UiSystem::Flex),
            )
            .add_system(
                widget::image_node_system
                    .stage(CoreStage::PostUpdate)
                    .before(UiSystem::Flex),
            )
            .add_system(
                flex_node_system
                    .stage(CoreStage::PostUpdate)
                    .label(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system(
                ui_z_system
                    .stage(CoreStage::PostUpdate)
                    .after(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system(widget::draw_text_system.stage(RenderStage::Draw));

        crate::render::add_ui_graph(&mut app.world);
    }
}
