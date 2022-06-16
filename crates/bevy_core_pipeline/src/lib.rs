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
use async_trait::async_trait;
use bevy_app::{App, Plugin};
use bevy_render::extract_resource::ExtractResourcePlugin;

#[derive(Default)]
pub struct CorePipelinePlugin;

#[async_trait]
impl Plugin for CorePipelinePlugin {
    async fn build(&self, app: &mut App) {
        app.init_resource::<ClearColor>()
            .add_plugin(ExtractResourcePlugin::<ClearColor>::default())
            .await
            .add_plugin(Core2dPlugin)
            .await
            .add_plugin(Core3dPlugin)
            .await;
    }
}
