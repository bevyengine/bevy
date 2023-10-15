use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut, Resource},
};
use bevy_math::{Mat4, Vec3, Vec3A};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    render_resource::{ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;

use crate::{
    environment_map::{self, RenderEnvironmentMaps},
    EnvironmentMapLight,
};

/// Adds support for light probes, cuboid bounding regions that apply global
/// illumination to objects within them.
pub struct LightProbePlugin;

/// A cuboid region that provides global illumination to all meshes inside it.
///
/// A mesh is considered inside the light probe if the mesh's origin is
/// contained within the cuboid centered at the light probe's transform with
/// width, height, and depth equal to double the value of `half_extents`.
///
/// Note that a light probe will have no effect unless the entity contains some
/// kind of illumination. At present, the only supported type of illumination is
/// the [EnvironmentMapLight].
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct LightProbe {
    /// The influence range of the light probe.
    pub half_extents: Vec3A,
}

#[derive(Clone, Copy, ShaderType, Default)]
pub struct GpuLightProbe {
    inverse_transform: Mat4,
    half_extents: Vec3,
    cubemap_index: i32,
}

#[derive(ShaderType)]
pub struct LightProbesUniform {
    data: [GpuLightProbe; 64],
    count: i32,
}

#[derive(Resource, Default)]
pub struct LightProbesBuffer {
    pub buffer: UniformBuffer<LightProbesUniform>,
}

impl LightProbe {
    /// Creates a new light probe component with the given half-extents.
    #[inline]
    pub fn new(half_extents: Vec3A) -> Self {
        Self { half_extents }
    }
}

impl Default for LightProbe {
    #[inline]
    fn default() -> Self {
        Self {
            half_extents: Vec3A::splat(1.0),
        }
    }
}

impl Plugin for LightProbePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LightProbe>();

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<LightProbesBuffer>()
            .add_systems(ExtractSchedule, gather_light_probes)
            .add_systems(
                Render,
                upload_light_probes
                    .in_set(RenderSet::PrepareResources)
                    .after(environment_map::prepare_environment_maps),
            );
    }
}

pub fn gather_light_probes(
    render_environment_maps: Res<RenderEnvironmentMaps>,
    light_probe_query: Extract<Query<(&LightProbe, &EnvironmentMapLight, &GlobalTransform)>>,
    mut light_probes_buffer: ResMut<LightProbesBuffer>,
) {
    // Gather up information about all light probes in the scene.
    let light_probes_uniform = light_probes_buffer.buffer.get_mut();
    light_probes_uniform.count = 0;
    for (light_probe, environment_map_light, light_probe_transform) in light_probe_query.iter() {
        if let Some(&cubemap_index) = render_environment_maps
            .light_id_indices
            .get(&environment_map_light.id())
        {
            light_probes_uniform.data[light_probes_uniform.count as usize] = GpuLightProbe {
                inverse_transform: light_probe_transform.compute_matrix().inverse(),
                half_extents: light_probe.half_extents.into(),
                cubemap_index,
            };
            light_probes_uniform.count += 1;
        }
    }
}

pub fn upload_light_probes(
    mut light_probes_buffer: ResMut<LightProbesBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    light_probes_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
}

impl Default for LightProbesUniform {
    fn default() -> Self {
        Self {
            data: [GpuLightProbe::default(); 64],
            count: i32::default(),
        }
    }
}
