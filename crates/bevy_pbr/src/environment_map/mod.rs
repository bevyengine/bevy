use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle, HandleUntyped};
use bevy_core_pipeline::prelude::Camera3d;
use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    system::{
        lifetimeless::{Read, SQuery},
        Resource, SystemParamItem,
    },
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        SamplerBindingType, Shader, ShaderStages, TextureSampleType, TextureViewDimension,
    },
    renderer::RenderDevice,
    texture::Image,
};

pub const ENVIRONMENT_BRDF_LUT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 754476556247605696);

pub struct EnvironmentMapPlugin;

impl Plugin for EnvironmentMapPlugin {
    fn build(&self, app: &mut App) {
        let lut_handle =
            load_internal_asset!(app, ENVIRONMENT_BRDF_LUT_HANDLE, "environment/brdf_lut.png");

        app.add_plugin(ExtractComponentPlugin::<EnvironmentMap>::default())
            .insert_resource(EnvironmentMapBRDFLUT { lut_handle });
    }
}

#[derive(Component, Clone)]
pub struct EnvironmentMap {
    bind_group: BindGroup,
}

impl EnvironmentMap {
    pub fn load_from_disk() -> Self {
        todo!()
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

#[derive(Resource)]
struct EnvironmentMapBRDFLUT {
    lut_handle: Handle<Image>,
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
    type Param = SQuery<Read<EnvironmentMap>>;
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
