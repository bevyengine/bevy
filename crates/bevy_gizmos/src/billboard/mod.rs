use std::{any::TypeId, mem};

use bevy_app::{App, Last, Plugin};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_color::LinearRgba;
use bevy_ecs::{
    component::Component,
    query::ROQueryItem,
    schedule::IntoSystemConfigs,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Res, ResMut, Resource, SystemParamItem,
    },
};
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{
        binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, Buffer, BufferInitDescriptor, BufferUsages, Shader, ShaderStages,
        ShaderType, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
    },
    renderer::RenderDevice,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::TypeIdMap;
use bytemuck::cast_slice;

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::GizmoStorage,
    UpdateGizmoMeshes,
};

#[cfg(all(feature = "bevy_sprite", feature = "bevy_render"))]
mod pipeline_2d;
#[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
mod pipeline_3d;

#[cfg(feature = "bevy_render")]
const BILLBOARD_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(6006413002665766670);

/// A [`Plugin`] that provides an immediate mode billboard drawing api for visual debugging.
#[derive(Default)]
pub struct BillboardGizmoPlugin;

impl Plugin for BillboardGizmoPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_render")]
        {
            use bevy_asset::load_internal_asset;
            load_internal_asset!(
                app,
                BILLBOARD_SHADER_HANDLE,
                "billboards.wgsl",
                Shader::from_wgsl
            );
        }
        app.init_asset::<BillboardGizmo>()
            .init_resource::<BillboardGizmoHandles>();

        #[cfg(feature = "bevy_render")]
        app.add_plugins(UniformComponentPlugin::<BillboardGizmoUniform>::default())
            .add_plugins(RenderAssetPlugin::<GpuBillboardGizmo>::default());

        #[cfg(feature = "bevy_render")]
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                Render,
                prepare_billboard_gizmo_bind_group.in_set(RenderSet::PrepareBindGroups),
            );

            render_app.add_systems(ExtractSchedule, extract_gizmo_data);

            #[cfg(feature = "bevy_sprite")]
            if app.is_plugin_added::<bevy_sprite::SpritePlugin>() {
                app.add_plugins(pipeline_2d::BillboardGizmo2dPlugin);
            } else {
                bevy_utils::tracing::warn!("bevy_sprite feature is enabled but bevy_sprite::SpritePlugin was not detected. Are you sure you loaded GizmoPlugin after SpritePlugin?");
            }
            #[cfg(feature = "bevy_pbr")]
            if app.is_plugin_added::<bevy_pbr::PbrPlugin>() {
                app.add_plugins(pipeline_3d::BillboardGizmo3dPlugin);
            } else {
                bevy_utils::tracing::warn!("bevy_pbr feature is enabled but bevy_pbr::PbrPlugin was not detected. Are you sure you loaded GizmoPlugin after PbrPlugin?");
            }
        } else {
            bevy_utils::tracing::warn!("bevy_render feature is enabled but RenderApp was not detected. Are you sure you loaded GizmoPlugin after RenderPlugin?");
        }
    }

    #[cfg(feature = "bevy_render")]
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world().resource::<RenderDevice>();
        let billboard_layout = render_device.create_bind_group_layout(
            "BillboardGizmoUniform layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX,
                uniform_buffer::<BillboardGizmoUniform>(true),
            ),
        );

        render_app.insert_resource(BillboardGizmoUniformBindgroupLayout {
            layout: billboard_layout,
        });
    }
}

/// An internal extension trait adding `App::init_billboard_gizmo_group`.
pub(crate) trait AppBillboardGizmoBuilder {
    /// Registers [`GizmoConfigGroup`] in the app enabling the use of [Gizmos&lt;Config&gt;](crate::gizmos::Gizmos).
    ///
    /// Configurations can be set using the [`GizmoConfigStore`] [`Resource`].
    fn init_billboard_gizmo_group<Config: GizmoConfigGroup>(&mut self) -> &mut Self;
}

impl AppBillboardGizmoBuilder for App {
    fn init_billboard_gizmo_group<Config: GizmoConfigGroup>(&mut self) -> &mut Self {
        if self.world().contains_resource::<GizmoStorage<Config, ()>>() {
            return self;
        }

        let mut handles = self
            .world_mut()
            .get_resource_or_insert_with::<BillboardGizmoHandles>(Default::default);

        handles.billboards.insert(TypeId::of::<Config>(), None);

        self.add_systems(
            Last,
            update_gizmo_meshes::<Config>.in_set(UpdateGizmoMeshes),
        );

        self
    }
}

/// Holds handles to the billboard gizmos for each gizmo configuration group
// As `TypeIdMap` iteration order depends on the order of insertions and deletions, this uses
// `Option<Handle>` to be able to reserve the slot when creating the gizmo configuration group.
// That way iteration order is stable across executions and depends on the order of configuration
// group creation.
#[derive(Resource, Default)]
struct BillboardGizmoHandles {
    billboards: TypeIdMap<Option<Handle<BillboardGizmo>>>,
}

/// Prepare gizmos for rendering.
///
/// This also clears the default `GizmoStorage`.
fn update_gizmo_meshes<Config: GizmoConfigGroup>(
    mut billboard_gizmos: ResMut<Assets<BillboardGizmo>>,
    mut handles: ResMut<BillboardGizmoHandles>,
    mut storage: ResMut<GizmoStorage<Config, ()>>,
) {
    if storage.list_positions.is_empty() {
        handles.billboards.insert(TypeId::of::<Config>(), None);
    } else if let Some(handle) = handles.billboards.get_mut(&TypeId::of::<Config>()) {
        if let Some(handle) = handle {
            let billboards = billboard_gizmos.get_mut(handle.id()).unwrap();

            billboards.positions = mem::take(&mut storage.billboard_positions);
            billboards.colors = mem::take(&mut storage.billboard_colors);
        } else {
            let billboards = BillboardGizmo {
                positions: mem::take(&mut storage.billboard_positions),
                colors: mem::take(&mut storage.billboard_colors),
            };

            *handle = Some(billboard_gizmos.add(billboards));
        }
    }
}

#[cfg(feature = "bevy_render")]
fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<BillboardGizmoHandles>>,
    config: Extract<Res<GizmoConfigStore>>,
) {
    for (group_type_id, handle) in handles.billboards.iter() {
        let Some((config, _)) = config.get_config_dyn(group_type_id) else {
            continue;
        };

        if !config.enabled {
            continue;
        }

        let Some(handle) = handle else {
            continue;
        };

        commands.spawn((
            BillboardGizmoUniform {
                size: config.billboard_size,
                depth_bias: config.depth_bias,
                #[cfg(feature = "webgl")]
                _padding: Default::default(),
            },
            (*handle).clone_weak(),
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite"))]
            crate::config::GizmoMeshConfig::from(config),
        ));
    }
}

#[cfg(feature = "bevy_render")]
#[derive(Component, ShaderType, Clone, Copy)]
struct BillboardGizmoUniform {
    size: f32,
    depth_bias: f32,
    /// WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl")]
    _padding: bevy_math::Vec2,
}

/// A gizmo asset that represents a billboard.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct BillboardGizmo {
    /// Positions of the gizmo's vertices
    pub positions: Vec<Vec3>,
    /// Colors of the gizmo's vertices
    pub colors: Vec<LinearRgba>,
}

#[cfg(feature = "bevy_render")]
#[derive(Debug, Clone)]
struct GpuBillboardGizmo {
    position_buffer: Buffer,
    color_buffer: Buffer,
    vertex_count: u32,
}

#[cfg(feature = "bevy_render")]
impl RenderAsset for GpuBillboardGizmo {
    type SourceAsset = BillboardGizmo;
    type Param = SRes<RenderDevice>;

    fn prepare_asset(
        gizmo: Self::SourceAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let position_buffer_data = cast_slice(&gizmo.positions);
        let position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("BillboardGizmo Position Buffer"),
            contents: position_buffer_data,
        });

        let color_buffer_data = cast_slice(&gizmo.colors);
        let color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("BillboardGizmo Color Buffer"),
            contents: color_buffer_data,
        });

        Ok(GpuBillboardGizmo {
            position_buffer,
            color_buffer,
            vertex_count: gizmo.positions.len() as u32,
        })
    }
}

#[cfg(feature = "bevy_render")]
#[derive(Resource)]
struct BillboardGizmoUniformBindgroupLayout {
    layout: BindGroupLayout,
}

#[cfg(feature = "bevy_render")]
#[derive(Resource)]
struct BillboardGizmoUniformBindgroup {
    bindgroup: BindGroup,
}

#[cfg(feature = "bevy_render")]
fn prepare_billboard_gizmo_bind_group(
    mut commands: Commands,
    billboard_gizmo_uniform_layout: Res<BillboardGizmoUniformBindgroupLayout>,
    render_device: Res<RenderDevice>,
    billboard_gizmo_uniforms: Res<ComponentUniforms<BillboardGizmoUniform>>,
) {
    if let Some(binding) = billboard_gizmo_uniforms.uniforms().binding() {
        commands.insert_resource(BillboardGizmoUniformBindgroup {
            bindgroup: render_device.create_bind_group(
                "BillboardGizmoUniform bindgroup",
                &billboard_gizmo_uniform_layout.layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

#[cfg(feature = "bevy_render")]
struct SetBillboardGizmoBindGroup<const I: usize>;
#[cfg(feature = "bevy_render")]
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetBillboardGizmoBindGroup<I> {
    type Param = SRes<BillboardGizmoUniformBindgroup>;
    type ViewQuery = ();
    type ItemQuery = Read<DynamicUniformIndex<BillboardGizmoUniform>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        uniform_index: Option<ROQueryItem<'w, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(uniform_index) = uniform_index else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(
            I,
            &bind_group.into_inner().bindgroup,
            &[uniform_index.index()],
        );
        RenderCommandResult::Success
    }
}

#[cfg(feature = "bevy_render")]
struct DrawBillboardGizmo;
#[cfg(feature = "bevy_render")]
impl<P: PhaseItem> RenderCommand<P> for DrawBillboardGizmo {
    type Param = SRes<RenderAssets<GpuBillboardGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<BillboardGizmo>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        billboard_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(handle) = handle else {
            return RenderCommandResult::Skip;
        };
        let Some(billboard_gizmo) = billboard_gizmos.into_inner().get(handle) else {
            return RenderCommandResult::Skip;
        };

        if billboard_gizmo.vertex_count == 0 {
            return RenderCommandResult::Success;
        }

        let instances = {
            pass.set_vertex_buffer(0, billboard_gizmo.position_buffer.slice(..));
            pass.set_vertex_buffer(1, billboard_gizmo.color_buffer.slice(..));

            billboard_gizmo.vertex_count
        };

        pass.draw(0..6, 0..instances);

        RenderCommandResult::Success
    }
}

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
fn billboard_gizmo_vertex_buffer_layouts() -> Vec<VertexBufferLayout> {
    use VertexFormat::*;
    let position_layout = VertexBufferLayout {
        array_stride: Float32x3.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x3,
            offset: 0,
            shader_location: 0,
        }],
    };

    let color_layout = VertexBufferLayout {
        array_stride: Float32x4.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x4,
            offset: 0,
            shader_location: 1,
        }],
    };

    vec![position_layout, color_layout]
}
