pub mod render_graph;

mod entity;
mod light;
mod material;

pub use entity::*;
pub use light::*;
pub use material::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        entity::*,
        light::{DirectionalLight, PointLight},
        material::StandardMaterial,
    };
}

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_render::{prelude::Color, shader};
use material::StandardMaterial;
use render_graph::add_pbr_graph;

/// NOTE: this isn't PBR yet. consider this name "aspirational" :)
#[derive(Default)]
pub struct PbrPlugin;

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<StandardMaterial>()
            .register_type::<PointLight>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                shader::asset_shader_defs_system::<StandardMaterial>,
            )
            .init_resource::<AmbientLight>();
        add_pbr_graph(&mut app.world);

        // add default StandardMaterial
        let mut materials = app
            .world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        materials.set_untracked(
            Handle::<StandardMaterial>::default(),
            StandardMaterial {
                base_color: Color::PINK,
                unlit: true,
                ..Default::default()
            },
        );
    }
}
