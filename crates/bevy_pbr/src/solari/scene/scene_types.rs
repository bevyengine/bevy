use crate::{solari::SolariEnabled, DirectionalLight, StandardMaterial};
use bevy_asset::{Assets, Handle};
use bevy_core::FrameCount;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Local, Query, Res},
};
use bevy_math::{Vec3, Vec4};
use bevy_render::{
    color::Color,
    render_resource::{ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    Extract,
};
use bevy_transform::components::GlobalTransform;

#[derive(Component)]
pub struct SolariMaterial {
    pub base_color: Color,
    pub base_color_texture: Option<Handle<Image>>,
    pub normal_map_texture: Option<Handle<Image>>,
    pub emissive: Color,
    pub emissive_texture: Option<Handle<Image>>,
}

#[derive(ShaderType)]
pub struct GpuSolariMaterial {
    pub base_color: Vec4,
    pub base_color_texture_index: u32,
    pub normal_map_texture_index: u32,
    pub emissive: Vec3,
    pub emissive_texture_index: u32,
}

#[derive(ShaderType)]
pub struct SolariUniforms {
    pub frame_count: u32,
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
}

impl SolariUniforms {
    pub fn new(
        frame_count: &FrameCount,
        sun: (&DirectionalLight, &GlobalTransform),
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> UniformBuffer<Self> {
        let sun_color = sun.0.color.as_linear_rgba_f32();
        let uniforms = Self {
            frame_count: frame_count.0,
            sun_direction: sun.1.back(),
            sun_color: Vec3::new(sun_color[0], sun_color[1], sun_color[2]) * sun.0.illuminance,
        };

        let mut buffer = UniformBuffer::from(uniforms);
        buffer.set_label(Some("solari_uniforms"));
        buffer.write_buffer(render_device, render_queue);
        buffer
    }
}

pub fn extract(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(Entity, &Handle<StandardMaterial>)>>,
    materials: Extract<Res<Assets<StandardMaterial>>>,
    solari_enabled: Extract<Option<Res<SolariEnabled>>>,
) {
    if solari_enabled.is_none() {
        return;
    }

    let mut entities = Vec::with_capacity(*previous_len);

    for (entity, material_handle) in &query {
        if let Some(material) = materials.get(material_handle) {
            let solari_material = SolariMaterial {
                base_color: material.base_color,
                base_color_texture: material.base_color_texture.clone(),
                normal_map_texture: material.normal_map_texture.clone(),
                emissive: material.emissive,
                emissive_texture: material.emissive_texture.clone(),
            };

            entities.push((entity, solari_material));
        }
    }

    *previous_len = entities.len();
    commands.insert_or_spawn_batch(entities);

    commands.insert_resource(SolariEnabled);
}
