pub mod clear_color;
pub mod core_2d;
pub mod core_3d;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        clear_color::ClearColor,
        core_2d::{Camera2d, Camera2dBundle},
        core_3d::{Camera3d, Camera3dBundle},
    };
}

use crate::{clear_color::ClearColor, core_2d::Core2dPlugin, core_3d::Core3dPlugin};
use bevy_app::{App, Plugin};
use bevy_render::extract_resource::ExtractResourcePlugin;

#[derive(Default)]
pub struct CorePipelinePlugin;

impl Plugin for CorePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ClearColor>()
            .init_resource::<ClearColor>()
            .add_plugin(ExtractResourcePlugin::<ClearColor>::default())
            .add_plugin(Core2dPlugin)
            .add_plugin(Core3dPlugin);
    }
}
