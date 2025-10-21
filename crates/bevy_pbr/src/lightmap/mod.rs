//! Lightmaps, baked lighting textures that can be applied at runtime to provide
//! diffuse global illumination.
//!
//! Bevy doesn't currently have any way to actually bake lightmaps, but they can
//! be baked in an external tool like [Blender](http://blender.org), for example
//! with an addon like [The Lightmapper]. The tools in the [`bevy-baked-gi`]
//! project support other lightmap baking methods.
//!
//! When a [`Lightmap`] component is added to an entity with a [`Mesh3d`] and a
//! [`MeshMaterial3d<StandardMaterial>`], Bevy applies the lightmap when rendering. The brightness
//! of the lightmap may be controlled with the `lightmap_exposure` field on
//! [`StandardMaterial`].
//!
//! During the rendering extraction phase, we extract all lightmaps into the
//! [`RenderLightmaps`] table, which lives in the render world. Mesh bindgroup
//! and mesh uniform creation consults this table to determine which lightmap to
//! supply to the shader. Essentially, the lightmap is a special type of texture
//! that is part of the mesh instance rather than part of the material (because
//! multiple meshes can share the same material, whereas sharing lightmaps is
//! nonsensical).
//!
//! Note that multiple meshes can't be drawn in a single drawcall if they use
//! different lightmap textures, unless bindless textures are in use. If you
//! want to instance a lightmapped mesh, and your platform doesn't support
//! bindless textures, combine the lightmap textures into a single atlas, and
//! set the `uv_rect` field on [`Lightmap`] appropriately.
//!
//! [The Lightmapper]: https://github.com/Naxela/The_Lightmapper
//! [`Mesh3d`]: bevy_mesh::Mesh3d
//! [`MeshMaterial3d<StandardMaterial>`]: crate::StandardMaterial
//! [`StandardMaterial`]: crate::StandardMaterial
//! [`bevy-baked-gi`]: https://github.com/pcwalton/bevy-baked-gi

use bevy_app::{App, Plugin};
use bevy_ecs::{
    schedule::IntoScheduleConfigs,
    system::{Commands, Res},
};
use bevy_render::renderer::RenderDevice;
use bevy_render::{renderer::RenderAdapter, ExtractSchedule, RenderApp, RenderStartup};
use bevy_shader::load_shader_library;
use bevy_utils::default;
use fixedbitset::FixedBitSet;

pub use bevy_render::lightmap::*;

use crate::{binding_arrays_are_usable, MeshExtractionSystems};

/// A plugin that provides an implementation of lightmaps.
pub struct LightmapPlugin;

impl Plugin for LightmapPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "lightmap.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_render_lightmaps)
            .add_systems(
                ExtractSchedule,
                extract_lightmaps.after(MeshExtractionSystems),
            );
    }
}

pub fn init_render_lightmaps(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
) {
    let bindless_supported = binding_arrays_are_usable(&render_device, &render_adapter);

    commands.insert_resource(RenderLightmaps {
        render_lightmaps: default(),
        slabs: vec![],
        free_slabs: FixedBitSet::new(),
        pending_lightmaps: default(),
        bindless_supported,
    });
}
