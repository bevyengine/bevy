use crate::{ui::Node, *};
use winit::window::Window;

pub fn build_ui_update_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("ui_update_system")
        .read_resource::<Window>()
        .with_query(<(Write<Node>,)>::query().filter(!component::<Children>()))
        .build(move |_, world, window, node_query| {
            let window_size = window.inner_size();
            let parent_dimensions = math::vec2(window_size.width as f32, window_size.height as f32);
            for (mut node,) in node_query.iter_mut(world) {
                node.update(parent_dimensions);
            }
        })
}
