mod anchors;
pub mod entity;
mod events;
mod flex;
mod focus;
mod margins;
mod node;
mod render;
pub mod update;
pub mod widget;

pub use anchors::*;
pub use events::*;
pub use flex::*;
pub use focus::*;
pub use margins::*;
pub use node::*;
pub use render::*;

pub mod prelude {
    pub use crate::{
        entity::*,
        events::*,
        node::*,
        widget::{Button, Text},
        Anchors, Interaction, Margins,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;
use bevy_render::render_graph::RenderGraph;
use update::ui_z_system;

#[derive(Default)]
pub struct UiPlugin;

pub mod stage {
    pub const UI: &str = "ui";
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<MouseDown>()
            .add_event::<MouseUp>()
            .add_event::<MouseEnter>()
            .add_event::<MouseHover>()
            .add_event::<MouseLeave>()
            .add_event::<Click>()
            .add_event::<DoubleClick>()
            .init_resource::<FlexSurface>()
            .add_stage_before(bevy_app::stage::POST_UPDATE, stage::UI)
            .add_system_to_stage(bevy_app::stage::PRE_UPDATE, ui_focus_system.system())
            .add_system_to_stage(bevy_app::stage::PRE_UPDATE, ui_event_system.system())
            // add these stages to front because these must run before transform update systems
            .add_system_to_stage(stage::UI, widget::text_system.system())
            .add_system_to_stage(stage::UI, widget::image_node_system.system())
            .add_system_to_stage(stage::UI, ui_z_system.system())
            .add_system_to_stage(stage::UI, flex_node_system.system())
            .add_system_to_stage(bevy_render::stage::DRAW, widget::draw_text_system.system());

        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_ui_graph(resources);
    }
}
