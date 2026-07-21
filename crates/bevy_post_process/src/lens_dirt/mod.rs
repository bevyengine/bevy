use bevy_app::Plugin;
use bevy_asset::Handle;
use bevy_camera::Camera;
use bevy_color::{Color, ColorToComponents};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Resource,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_image::Image;
use bevy_math::Vec3;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, SamplerBindingType, ShaderStages, ShaderType, TextureSampleType,
    },
    renderer::RenderDevice,
    sync_component::SyncComponent,
    texture::{FallbackImage, GpuImage},
    uniform::{ComponentUniforms, UniformComponentPlugin},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::load_shader_library;

/// A plugin that adds support for the lens dirt effect to Bevy.
#[derive(Default)]
pub struct LensDirtPlugin;

impl Plugin for LensDirtPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_shader_library!(app, "lens_dirt.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<LensDirt>::default(),
            UniformComponentPlugin::<LensDirtUniforms>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_lens_dirt_bind_group)
            .add_systems(
                Render,
                prepare_lens_dirt_bind_group.in_set(RenderSystems::PrepareBindGroups),
            );
    }
}

#[derive(Resource)]
pub struct LensDirtBindGroupLayout(pub BindGroupLayout);

#[derive(Component)]
pub struct LensDirtBindGroup(pub BindGroup);

pub fn create_lens_dirt_bind_group_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "lens_dirt_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<LensDirtUniforms>(true),
            ),
        ),
    )
}

fn init_lens_dirt_bind_group(mut commands: Commands, render_device: Res<RenderDevice>) {
    let bind_group_layout = render_device.create_bind_group_layout(
        "lens_dirt_bind_group_layout",
        &create_lens_dirt_bind_group_layout().entries,
    );

    commands.insert_resource(LensDirtBindGroupLayout(bind_group_layout));
}

fn prepare_lens_dirt_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    lens_dirt_layout: Res<LensDirtBindGroupLayout>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    uniforms: Res<ComponentUniforms<LensDirtUniforms>>,
    fallback: Res<FallbackImage>,
    views: Query<(Entity, &LensDirt)>,
) {
    let Some(uniform_binding) = uniforms.binding() else {
        return;
    };
    for (entity, lens_dirt) in &views {
        let dirt_image = gpu_images.get(&lens_dirt.texture).unwrap_or(&fallback.d2);
        let bind_group = render_device.create_bind_group(
            "lens_dirt_bind_group",
            &lens_dirt_layout.0,
            &BindGroupEntries::sequential((
                &dirt_image.texture_view,
                &dirt_image.sampler,
                uniform_binding.clone(),
            )),
        );
        commands
            .entity(entity)
            .insert(LensDirtBindGroup(bind_group));
    }
}

/// A component that enables a lens dirt effect when added to a camera.
/// Simulates the effect of dirt on the lens.
///
/// Currently, the lens dirt only interacts with the bloom effect.
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct LensDirt {
    /// The lens dirt texture. Set to `Some` to enable the effect.
    pub texture: Handle<Image>,

    /// How strongly the lens dirt appears (default: 1.0).
    ///
    /// Valid range: 0.0 to 1.0 where:
    /// * 0.0 - No dirt visible
    /// * 1.0 - Full dirt intensity
    pub intensity: f32,

    /// Color tint applied to the lens dirt (default: `Color::WHITE`).
    ///
    /// Use this to match the dirt effect to your scene's lighting or mood.
    pub tint: Color,
}

impl Default for LensDirt {
    fn default() -> Self {
        Self {
            texture: Handle::default(),
            intensity: 1.0,
            tint: Color::default(),
        }
    }
}

impl SyncComponent<RenderApp> for LensDirt {
    type Target = (Self, LensDirt);
}

impl ExtractComponent<RenderApp> for LensDirt {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;
    type Out = (Self, LensDirtUniforms);

    fn extract_component(lens_dirt: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some((
            lens_dirt.clone(),
            LensDirtUniforms {
                intensity: lens_dirt.intensity,
                tint: lens_dirt.tint.to_linear().to_vec3(),
            },
        ))
    }
}

/// The uniform struct extracted from [`LensDirt`] attached to a Camera.
#[derive(Component, ShaderType, Clone)]
pub struct LensDirtUniforms {
    pub intensity: f32,
    pub tint: Vec3,
}
