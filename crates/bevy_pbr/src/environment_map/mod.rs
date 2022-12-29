use crate::MeshPipeline;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, load_internal_binary_asset, Handle, HandleUntyped};
use bevy_core_pipeline::prelude::Camera3d;
use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    system::{
        lifetimeless::{Read, SQuery},
        Commands, Query, Res, SystemParamItem,
    },
};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, SamplerBindingType, Shader,
        ShaderStages, TextureSampleType, TextureViewDimension,
    },
    renderer::RenderDevice,
    texture::{CompressedImageFormats, Image, ImageType},
    RenderApp, RenderStage,
};

pub const ENVIRONMENT_MAP_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 154476556247605696);
pub const ENVIRONMENT_BRDF_LUT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Image::TYPE_UUID, 754476556247605696);

pub struct EnvironmentMapPlugin;

impl Plugin for EnvironmentMapPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            ENVIRONMENT_MAP_SHADER_HANDLE,
            "environment_map.wgsl",
            Shader::from_wgsl
        );
        load_internal_binary_asset!(app, ENVIRONMENT_BRDF_LUT_HANDLE, "brdf_lut.png", |bytes| {
            Image::from_buffer(
                bytes,
                ImageType::MimeType("image/png"),
                CompressedImageFormats::NONE,
                false,
            )
            .unwrap()
        });

        app.register_type::<EnvironmentMap>()
            .add_plugin(ExtractComponentPlugin::<EnvironmentMap>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };

        render_app.add_system_to_stage(RenderStage::Queue, queue_environment_map_bind_groups);
    }
}

/// Environment map based indirect lighting.
///
/// When added to a 3D camera, this component adds indirect light
/// to the scene based on an environment cubemap texture.
///
/// The environment map must be prefiltered into a diffuse and specular map based on the
/// [split-sum approximation](https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf).
/// The specular map must have exactly 11 mips [0, 10].
///
/// To prefilter your environment map, you can use KhronosGroup's [glTF-IBL-Sampler](https://github.com/KhronosGroup/glTF-IBL-Sampler).
/// The diffuse map uses the Lambertian distribution, and the specular map uses the GGX distribution.
///
/// KhronoGroup also has several prefiltered environment maps that can be found [here](https://github.com/KhronosGroup/glTF-Sample-Environments).
#[derive(Component, Reflect, Clone)]
pub struct EnvironmentMap {
    pub diffuse_map: Handle<Image>,
    pub specular_map: Handle<Image>,
}

impl EnvironmentMap {
    pub fn is_ready(&self, images: &RenderAssets<Image>) -> bool {
        images.get(&ENVIRONMENT_BRDF_LUT_HANDLE.typed()).is_some()
            && images.get(&self.diffuse_map).is_some()
            && images.get(&self.specular_map).is_some()
    }
}

impl ExtractComponent for EnvironmentMap {
    type Query = &'static Self;
    type Filter = With<Camera3d>;
    type Out = Self;

    fn extract_component(item: bevy_ecs::query::QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

#[derive(Component)]
pub struct EnvironmentMapBindGroup {
    bind_group: BindGroup,
}

fn queue_environment_map_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    pipeline: Res<MeshPipeline>,
    views: Query<(Entity, &EnvironmentMap)>,
) {
    for (entity, environment_map) in &views {
        let (Some(brdf_lut), Some(diffuse_map), Some(specular_map)) = (
            images.get(&ENVIRONMENT_BRDF_LUT_HANDLE.typed()),
            images.get(&environment_map.diffuse_map),
            images.get(&environment_map.specular_map)
        ) else { return };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("environment_map_bind_group"),
            layout: &pipeline.environment_map_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&brdf_lut.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&diffuse_map.texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&specular_map.texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&brdf_lut.sampler),
                },
            ],
        });

        commands
            .entity(entity)
            .insert(EnvironmentMapBindGroup { bind_group });
    }
}

pub fn new_environment_map_bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("environment_map_bind_group_layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub struct SetMeshViewEnvironmentMapBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetMeshViewEnvironmentMapBindGroup<I> {
    type Param = SQuery<Read<EnvironmentMapBindGroup>>;
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        view_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let environment_map = view_query.get_inner(view).unwrap();

        pass.set_bind_group(I, &environment_map.bind_group, &[]);

        RenderCommandResult::Success
    }
}
