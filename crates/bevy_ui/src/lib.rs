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
    pub use crate::{
        entity::*,
        widget::{Button, Text},
        Anchors, Click, Hover, Margins, Node,
    };

    pub use stretch::{
        geometry::{Point, Rect, Size},
        style::{Style as Flex, *},
    };
}

use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;
use bevy_render::render_graph::RenderGraph;
use update::ui_z_system;

#[derive(Default)]
pub struct UiPlugin;

impl AppPlugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<FlexSurfaces>()
            .add_system_to_stage(stage::PRE_UPDATE, ui_focus_system.system())
            // add these stages to front because these must run before transform update systems
            .add_system_to_stage_front(stage::POST_UPDATE, flex_node_system.system())
            .add_system_to_stage_front(stage::POST_UPDATE, ui_z_system.system())
            .add_system_to_stage_front(
                stage::POST_UPDATE,
                primary_window_flex_surface_system.system(),
            )
            .add_system_to_stage(stage::POST_UPDATE, widget::text_system.system())
            .add_system_to_stage(bevy_render::stage::DRAW, widget::draw_text_system.system());

        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_ui_graph(resources);
    }
}
