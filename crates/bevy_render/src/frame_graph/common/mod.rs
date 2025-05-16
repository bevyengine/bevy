pub mod bind_group;
pub mod color_attachment;
pub mod compute_pass_info;
pub mod depth_stencil_attachment;
pub mod render_pass_info;
pub mod resource_meta;
pub mod texel_copy_texture_info;
pub mod texture_view;

pub use bind_group::*;
pub use color_attachment::*;
pub use compute_pass_info::*;
pub use depth_stencil_attachment::*;
pub use render_pass_info::*;
pub use resource_meta::*;
pub use texel_copy_texture_info::*;
pub use texture_view::*;

use super::RenderContext;

pub trait ResourceBinding {
    type Resource;

    fn make_resource<'a>(&self, render_context: &RenderContext<'a>) -> Self::Resource;
}
