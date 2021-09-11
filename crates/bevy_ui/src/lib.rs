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
    pub use crate::{
        entity::*, ui_node::*, widget::Button, Anchors, Interaction, Margins, UiScale,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::schedule::{ParallelSystemDescriptorCoercion, SystemLabel};
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

#[derive(Debug)]
/// The current scale of the UI for all windows
///
/// ## Note
/// This is purely about the logical scale, and can
/// be considered like a zoom
///
/// This only affects pixel sizes, so a percent size will stay at that
pub struct UiScale {
    /// The scale to be applied
    ///
    /// # Example
    ///
    /// A scale of `2.` will make every pixel size twice as large.
    pub scale: f64,
}

impl Default for UiScale {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlexSurface>()
            .init_resource::<UiScale>()
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
            .add_system_to_stage(
                CoreStage::PreUpdate,
                ui_focus_system.label(UiSystem::Focus).after(InputSystem),
            )
            // add these stages to front because these must run before transform update systems
            .add_system_to_stage(
                CoreStage::PostUpdate,
                widget::text_system.before(UiSystem::Flex),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                widget::image_node_system.before(UiSystem::Flex),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                flex_node_system
                    .label(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                ui_z_system
                    .after(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(RenderStage::Draw, widget::draw_text_system);

        crate::render::add_ui_graph(&mut app.world);
    }
}
