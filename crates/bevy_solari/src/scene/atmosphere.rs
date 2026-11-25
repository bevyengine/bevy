use bevy_ecs::{
    resource::Resource,
    system::{Res, SystemParam},
    world::{FromWorld, World},
};
use bevy_pbr::{
    resources::{AtmosphereTextures, GpuAtmosphere},
    LinearSampler,
};
use bevy_render::{
    render_resource::{
        BindingResource, Buffer, BufferInitDescriptor, BufferUsages, Sampler, ShaderType,
        TextureView,
    },
    renderer::RenderDevice,
    texture::FallbackImage,
    uniform::{ComponentUniforms, DynamicUniformIndex},
};

/// Zeroed atmosphere uniform for views without an atmosphere.
#[derive(Resource)]
pub struct AtmosphereFallbackUniform(Buffer);

impl FromWorld for AtmosphereFallbackUniform {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        Self(
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("solari_atmosphere_fallback"),
                contents: &vec![0; GpuAtmosphere::min_size().get() as usize],
                usage: BufferUsages::UNIFORM,
            }),
        )
    }
}

/// Binds a view's atmosphere for `scene/atmosphere.wesl`.
#[derive(SystemParam)]
pub struct AtmosphereBindings<'w> {
    uniforms: Option<Res<'w, ComponentUniforms<GpuAtmosphere>>>,
    linear_sampler: Res<'w, LinearSampler>,
    fallback_image: Res<'w, FallbackImage>,
    fallback_uniform: Res<'w, AtmosphereFallbackUniform>,
}

/// Bindings 10-12 and the uniform's dynamic offset.
pub struct ViewAtmosphereBindings<'a> {
    pub transmittance_lut: &'a TextureView,
    pub sampler: &'a Sampler,
    pub uniform: BindingResource<'a>,
    pub offset: u32,
}

impl AtmosphereBindings<'_> {
    /// Falls back to a zeroed uniform for views without an atmosphere.
    pub fn view<'a>(
        &'a self,
        textures: Option<&'a AtmosphereTextures>,
        uniform_index: Option<&DynamicUniformIndex<GpuAtmosphere>>,
    ) -> ViewAtmosphereBindings<'a> {
        let (transmittance_lut, uniform, offset) = match (
            textures,
            uniform_index,
            self.uniforms.as_ref().and_then(|u| u.binding()),
        ) {
            (Some(textures), Some(uniform_index), Some(uniform)) => (
                &textures.transmittance_lut.default_view,
                uniform,
                uniform_index.index(),
            ),
            _ => (
                &self.fallback_image.d2.texture_view,
                self.fallback_uniform.0.as_entire_binding(),
                0,
            ),
        };
        ViewAtmosphereBindings {
            transmittance_lut,
            sampler: &self.linear_sampler.0,
            uniform,
            offset,
        }
    }
}
