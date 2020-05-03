mod anchors;
pub mod entity;
mod margins;
mod node;
mod rect;
mod render;
mod ui_update_system;

pub use anchors::*;
pub use margins::*;
pub use node::*;
pub use rect::*;
pub use render::*;
pub use ui_update_system::*;

use bevy_app::{AppBuilder, AppPlugin};
use bevy_render::{mesh::{shape::Quad, Mesh}, render_graph::RenderGraph};
use bevy_asset::{AssetStorage, Handle};
use glam::Vec2;

#[derive(Default)]
pub struct UiPlugin;

pub const QUAD_HANDLE: Handle<Mesh> = Handle::from_bytes([179, 41, 129, 128, 95, 217, 79, 194, 167, 95, 107, 115, 97, 151, 20, 62]);

impl AppPlugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {

        {
            let mut render_graph = app.resources().get_mut::<RenderGraph>().unwrap();
            render_graph.add_ui_graph(app.resources());

            let mut meshes = app.resources().get_mut::<AssetStorage<Mesh>>().unwrap();
            meshes.add_with_handle(QUAD_HANDLE, Mesh::from(Quad {
                size: Vec2::new(1.0, 1.0),
            }));
        }
        app.add_system(ui_update_system());
    }
}
