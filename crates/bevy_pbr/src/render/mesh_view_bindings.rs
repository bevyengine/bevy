use crate::{
    GlobalLightMeta, GpuLights, GpuPointLights, LightMeta, ShadowPipeline, ViewClusterBindings,
    ViewLightsUniformOffset, ViewShadowBindings, CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT,
};
use bevy_app::App;
use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{
    lifetimeless::{Read, SQuery, SRes},
    SystemParamItem,
};
use bevy_render::render_resource::*;
use bevy_render::{
    auto_binding::{
        AddAutoBinding, AutoBindGroup, AutoBindGroupLayoutEntry, AutoBinding, ShaderBindingName,
    },
    render_resource::{BindingType, BufferBindingType, OwnedBindingResource, ShaderStages},
    renderer::RenderDevice,
};

pub trait AddPbrViewBindings {
    fn add_pbr_view_bindings<G: AutoBindGroup>(&mut self) -> &mut Self;
}

impl AddPbrViewBindings for App {
    fn add_pbr_view_bindings<G: AutoBindGroup>(&mut self) -> &mut Self {
        self.add_auto_binding::<G, GpuLightsBinding>()
            .add_auto_binding::<G, PointShadowTexturesBinding>()
            .add_auto_binding::<G, PointShadowSamplerBinding>()
            .add_auto_binding::<G, DirectionalShadowTexturesBinding>()
            .add_auto_binding::<G, DirectionalShadowSamplerBinding>()
            .add_auto_binding::<G, PointLightsBinding>()
            .add_auto_binding::<G, ClusteredLightIndexListsBinding>()
            .add_auto_binding::<G, ClusterOffsetsAndCountsBinding>()
    }
}

pub struct GpuLightsBinding;
impl AutoBinding for GpuLightsBinding {
    type LayoutParam = ();
    type BindingParam = (SRes<LightMeta>, SQuery<Read<ViewLightsUniformOffset>>);

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new("bevy_pbr::mesh_view_bindings", "lights")
    }
    fn bindgroup_layout_entry(_: SystemParamItem<Self::LayoutParam>) -> AutoBindGroupLayoutEntry {
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(GpuLights::min_size()),
            },
            count: None,
        }
    }
    fn binding_source(
        entity: Entity,
        (light_meta, offsets): &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        let Ok(dynamic_offset) = offsets.get(entity) else { return None };

        light_meta
            .view_gpu_lights
            .owned_binding(dynamic_offset.offset)
    }
}

pub struct PointShadowTexturesBinding;
impl AutoBinding for PointShadowTexturesBinding {
    type LayoutParam = ();
    type BindingParam = SQuery<Read<ViewShadowBindings>>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new("bevy_pbr::mesh_view_bindings", "point_shadow_textures")
    }
    fn bindgroup_layout_entry(_: SystemParamItem<Self::LayoutParam>) -> AutoBindGroupLayoutEntry {
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Depth,
                #[cfg(not(feature = "webgl"))]
                view_dimension: TextureViewDimension::CubeArray,
                #[cfg(feature = "webgl")]
                view_dimension: TextureViewDimension::Cube,
            },
            count: None,
        }
    }
    fn binding_source(
        entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        param
            .get(entity)
            .ok()
            .map(|b| OwnedBindingResource::TextureView(b.point_light_depth_texture_view.clone()))
    }
}

pub struct PointShadowSamplerBinding;
impl AutoBinding for PointShadowSamplerBinding {
    type LayoutParam = ();
    type BindingParam = SRes<ShadowPipeline>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new(
            "bevy_pbr::mesh_view_bindings",
            "point_shadow_textures_sampler",
        )
    }
    fn bindgroup_layout_entry(_: SystemParamItem<Self::LayoutParam>) -> AutoBindGroupLayoutEntry {
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Comparison),
            count: None,
        }
    }
    fn binding_source(
        _entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        Some(OwnedBindingResource::Sampler(
            param.point_light_sampler.clone(),
        ))
    }
}

pub struct DirectionalShadowTexturesBinding;
impl AutoBinding for DirectionalShadowTexturesBinding {
    type LayoutParam = ();
    type BindingParam = SQuery<Read<ViewShadowBindings>>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new(
            "bevy_pbr::mesh_view_bindings",
            "directional_shadow_textures",
        )
    }
    fn bindgroup_layout_entry(_: SystemParamItem<Self::LayoutParam>) -> AutoBindGroupLayoutEntry {
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Depth,
                #[cfg(not(feature = "webgl"))]
                view_dimension: TextureViewDimension::D2Array,
                #[cfg(feature = "webgl")]
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        }
    }
    fn binding_source(
        entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        param.get(entity).ok().map(|b| {
            OwnedBindingResource::TextureView(b.directional_light_depth_texture_view.clone())
        })
    }
}

pub struct DirectionalShadowSamplerBinding;
impl AutoBinding for DirectionalShadowSamplerBinding {
    type LayoutParam = ();
    type BindingParam = SRes<ShadowPipeline>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new(
            "bevy_pbr::mesh_view_bindings",
            "directional_shadow_textures_sampler",
        )
    }
    fn bindgroup_layout_entry(_: SystemParamItem<Self::LayoutParam>) -> AutoBindGroupLayoutEntry {
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Comparison),
            count: None,
        }
    }
    fn binding_source(
        _entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        Some(OwnedBindingResource::Sampler(
            param.directional_light_sampler.clone(),
        ))
    }
}

pub struct PointLightsBinding;
impl AutoBinding for PointLightsBinding {
    type LayoutParam = SRes<RenderDevice>;
    type BindingParam = SRes<GlobalLightMeta>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new("bevy_pbr::mesh_view_bindings", "point_lights")
    }
    fn bindgroup_layout_entry(
        param: SystemParamItem<Self::LayoutParam>,
    ) -> AutoBindGroupLayoutEntry {
        let clustered_forward_buffer_binding_type =
            param.get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(GpuPointLights::min_size(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        }
    }
    fn binding_source(
        _entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        param
            .gpu_point_lights
            .buffer()
            .map(OwnedBindingResource::new_from_buffer)
    }
}

pub struct ClusteredLightIndexListsBinding;
impl AutoBinding for ClusteredLightIndexListsBinding {
    type LayoutParam = SRes<RenderDevice>;
    type BindingParam = SQuery<Read<ViewClusterBindings>>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new("bevy_pbr::mesh_view_bindings", "cluster_light_index_lists")
    }
    fn bindgroup_layout_entry(
        param: SystemParamItem<Self::LayoutParam>,
    ) -> AutoBindGroupLayoutEntry {
        let clustered_forward_buffer_binding_type =
            param.get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(ViewClusterBindings::min_size_cluster_light_index_lists(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        }
    }
    fn binding_source(
        entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        param
            .get(entity)
            .ok()
            .and_then(|clusters| clusters.light_index_lists_buffer())
            .map(OwnedBindingResource::new_from_buffer)
    }
}

pub struct ClusterOffsetsAndCountsBinding;
impl AutoBinding for ClusterOffsetsAndCountsBinding {
    type LayoutParam = SRes<RenderDevice>;
    type BindingParam = SQuery<Read<ViewClusterBindings>>;

    fn bind_name() -> ShaderBindingName {
        ShaderBindingName::new("bevy_pbr::mesh_view_bindings", "cluster_offsets_and_counts")
    }
    fn bindgroup_layout_entry(
        param: SystemParamItem<Self::LayoutParam>,
    ) -> AutoBindGroupLayoutEntry {
        let clustered_forward_buffer_binding_type =
            param.get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);
        AutoBindGroupLayoutEntry {
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(ViewClusterBindings::min_size_cluster_offsets_and_counts(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        }
    }
    fn binding_source(
        entity: Entity,
        param: &SystemParamItem<Self::BindingParam>,
    ) -> Option<OwnedBindingResource> {
        param
            .get(entity)
            .ok()
            .and_then(|clusters| clusters.offsets_and_counts_buffer())
            .map(OwnedBindingResource::new_from_buffer)
    }
}
