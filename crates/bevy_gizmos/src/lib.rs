#![allow(clippy::type_complexity)]
#![warn(missing_docs)]

//! This crate adds an immediate mode drawing api to Bevy for visual debugging.
//!
//! # Example
//! ```
//! # use bevy_gizmos::prelude::*;
//! # use bevy_render::prelude::*;
//! # use bevy_math::prelude::*;
//! fn system(mut gizmos: Gizmos) {
//!     gizmos.line(Vec3::ZERO, Vec3::X, Color::GREEN);
//! }
//! # bevy_ecs::system::assert_is_system(system);
//! ```
//!
//! See the documentation on [`Gizmos`] for more examples.

pub mod gizmos;

pub mod config;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

/// The `bevy_gizmos` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        config::{AabbGizmoConfig, CustomGizmoConfig, DefaultGizmoConfig, GizmoConfig},
        gizmos::Gizmos,
        AppGizmoBuilder, ShowAabbGizmo,
    };
}

use bevy_app::{App, Last, Plugin, PostUpdate};
use bevy_asset::{load_internal_asset, Asset, AssetApp, Assets, Handle};
use bevy_core::cast_slice;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{ROQueryItem, Without},
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Query, Res, ResMut, Resource, SystemParamItem,
    },
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypePath};
use bevy_render::{
    color::Color,
    extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
    primitives::Aabb,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferInitDescriptor,
        BufferUsages, Shader, ShaderStages, ShaderType, VertexAttribute, VertexBufferLayout,
        VertexFormat, VertexStepMode,
    },
    renderer::RenderDevice,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::{
    components::{GlobalTransform, Transform},
    TransformSystem,
};
use bevy_utils::HashMap;
use config::{AabbGizmoConfig, CustomGizmoConfig, DefaultGizmoConfig, GizmoConfigStore};
use gizmos::{GizmoStorage, Gizmos};
use std::{any::TypeId, mem};

const LINE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(7414812689238026784);

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, LINE_SHADER_HANDLE, "lines.wgsl", Shader::from_wgsl);

        app.add_plugins(UniformComponentPlugin::<LineGizmoUniform>::default())
            .init_asset::<LineGizmo>()
            .add_plugins(RenderAssetPlugin::<LineGizmo>::default())
            .init_resource::<LineGizmoHandles>()
            .init_resource::<GizmoConfigStore>()
            .init_gizmo_config::<DefaultGizmoConfig>()
            .init_gizmo_config::<AabbGizmoConfig>()
            .add_systems(
                PostUpdate,
                (
                    draw_aabbs,
                    draw_all_aabbs.run_if(|config: Res<AabbGizmoConfig>| config.draw_all),
                )
                    .after(TransformSystem::TransformPropagate),
            );

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            prepare_line_gizmo_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        #[cfg(feature = "bevy_sprite")]
        app.add_plugins(pipeline_2d::LineGizmo2dPlugin);
        #[cfg(feature = "bevy_pbr")]
        app.add_plugins(pipeline_3d::LineGizmo3dPlugin);
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(LineGizmoUniform::min_size()),
                },
                count: None,
            }],
            label: Some("LineGizmoUniform layout"),
        });

        render_app.insert_resource(LineGizmoUniformBindgroupLayout { layout });
    }
}

/// A trait adding `add_gizmos<T>()` to the app
pub trait AppGizmoBuilder {
    /// Adds a [`GizmoConfig<T>`] to the app enabling the use of [`Gizmos<T>`].
    fn init_gizmo_config<T: CustomGizmoConfig>(&mut self) -> &mut Self;
}

impl AppGizmoBuilder for App {
    fn init_gizmo_config<T: CustomGizmoConfig>(&mut self) -> &mut Self {
        self.init_resource::<T>()
            .init_resource::<GizmoStorage<T>>()
            .add_systems(Last, update_gizmo_meshes::<T>);

        self.world
            .resource_mut::<GizmoConfigStore>()
            .regsiter::<T>();

        let Ok(render_app) = self.get_sub_app_mut(RenderApp) else {
            return self;
        };

        render_app.add_systems(ExtractSchedule, extract_gizmo_data::<T>);

        self
    }
}

/// Add this [`Component`] to an entity to draw its [`Aabb`] component.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default)]
pub struct ShowAabbGizmo {
    /// The color of the box.
    ///
    /// The default color from the [`GizmoConfig`] resource is used if `None`,
    pub color: Option<Color>,
}

fn draw_aabbs(
    query: Query<(Entity, &Aabb, &GlobalTransform, &ShowAabbGizmo)>,
    config: Res<AabbGizmoConfig>,
    mut gizmos: Gizmos<AabbGizmoConfig>,
) {
    for (entity, &aabb, &transform, gizmo) in &query {
        let color = gizmo
            .color
            .or(config.default_color)
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.cuboid(aabb_transform(aabb, transform), color);
    }
}

fn draw_all_aabbs(
    query: Query<(Entity, &Aabb, &GlobalTransform), Without<ShowAabbGizmo>>,
    config: Res<AabbGizmoConfig>,
    mut gizmos: Gizmos<AabbGizmoConfig>,
) {
    for (entity, &aabb, &transform) in &query {
        let color = config
            .default_color
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.cuboid(aabb_transform(aabb, transform), color);
    }
}

fn color_from_entity(entity: Entity) -> Color {
    let index = entity.index();

    // from https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
    //
    // See https://en.wikipedia.org/wiki/Low-discrepancy_sequence
    // Map a sequence of integers (eg: 154, 155, 156, 157, 158) into the [0.0..1.0] range,
    // so that the closer the numbers are, the larger the difference of their image.
    const FRAC_U32MAX_GOLDEN_RATIO: u32 = 2654435769; // (u32::MAX / Φ) rounded up
    const RATIO_360: f32 = 360.0 / u32::MAX as f32;
    let hue = index.wrapping_mul(FRAC_U32MAX_GOLDEN_RATIO) as f32 * RATIO_360;

    Color::hsl(hue, 1., 0.5)
}

fn aabb_transform(aabb: Aabb, transform: GlobalTransform) -> GlobalTransform {
    transform
        * GlobalTransform::from(
            Transform::from_translation(aabb.center.into())
                .with_scale((aabb.half_extents * 2.).into()),
        )
}

#[derive(Resource, Default)]
struct LineGizmoHandles {
    list: HashMap<TypeId, Handle<LineGizmo>>,
    strip: HashMap<TypeId, Handle<LineGizmo>>,
}

fn update_gizmo_meshes<T: CustomGizmoConfig>(
    mut line_gizmos: ResMut<Assets<LineGizmo>>,
    mut handles: ResMut<LineGizmoHandles>,
    mut storage: ResMut<GizmoStorage<T>>,
) {
    if storage.list_positions.is_empty() {
        handles.list.remove(&TypeId::of::<T>());
    } else if let Some(handle) = handles.list.get(&TypeId::of::<T>()) {
        let list = line_gizmos.get_mut(handle).unwrap();

        list.positions = mem::take(&mut storage.list_positions);
        list.colors = mem::take(&mut storage.list_colors);
    } else {
        let mut list = LineGizmo {
            strip: false,
            ..Default::default()
        };

        list.positions = mem::take(&mut storage.list_positions);
        list.colors = mem::take(&mut storage.list_colors);

        handles
            .list
            .insert(TypeId::of::<T>(), line_gizmos.add(list));
    }

    if storage.strip_positions.is_empty() {
        handles.strip.remove(&TypeId::of::<T>());
    } else if let Some(handle) = handles.strip.get(&TypeId::of::<T>()) {
        let strip = line_gizmos.get_mut(handle).unwrap();

        strip.positions = mem::take(&mut storage.strip_positions);
        strip.colors = mem::take(&mut storage.strip_colors);
    } else {
        let mut strip = LineGizmo {
            strip: true,
            ..Default::default()
        };

        strip.positions = mem::take(&mut storage.strip_positions);
        strip.colors = mem::take(&mut storage.strip_colors);

        handles
            .strip
            .insert(TypeId::of::<T>(), line_gizmos.add(strip));
    }
}

fn extract_gizmo_data<T: CustomGizmoConfig>(
    mut commands: Commands,
    handles: Extract<Res<LineGizmoHandles>>,
    config: Extract<Res<GizmoConfigStore>>,
) {
    let config = config.get::<T>();

    if !config.enabled {
        return;
    }

    for map in [&handles.list, &handles.strip].into_iter() {
        let Some(handle) = map.get(&TypeId::of::<T>()) else {
            continue;
        };
        commands.spawn((
            LineGizmoUniform {
                line_width: config.line_width,
                depth_bias: config.depth_bias,
                #[cfg(feature = "webgl")]
                _padding: Default::default(),
            },
            (*handle).clone_weak(),
            config.clone(),
        ));
    }
}

#[derive(Component, ShaderType, Clone, Copy)]
struct LineGizmoUniform {
    line_width: f32,
    depth_bias: f32,
    /// WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl")]
    _padding: bevy_math::Vec2,
}

#[derive(Asset, Debug, Default, Clone, TypePath)]
struct LineGizmo {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    /// Whether this gizmo's topology is a line-strip or line-list
    strip: bool,
}

#[derive(Debug, Clone)]
struct GpuLineGizmo {
    position_buffer: Buffer,
    color_buffer: Buffer,
    vertex_count: u32,
    strip: bool,
}

impl RenderAsset for LineGizmo {
    type ExtractedAsset = LineGizmo;

    type PreparedAsset = GpuLineGizmo;

    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        line_gizmo: Self::ExtractedAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let position_buffer_data = cast_slice(&line_gizmo.positions);
        let position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Position Buffer"),
            contents: position_buffer_data,
        });

        let color_buffer_data = cast_slice(&line_gizmo.colors);
        let color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Color Buffer"),
            contents: color_buffer_data,
        });

        Ok(GpuLineGizmo {
            position_buffer,
            color_buffer,
            vertex_count: line_gizmo.positions.len() as u32,
            strip: line_gizmo.strip,
        })
    }
}

#[derive(Resource)]
struct LineGizmoUniformBindgroupLayout {
    layout: BindGroupLayout,
}

#[derive(Resource)]
struct LineGizmoUniformBindgroup {
    bindgroup: BindGroup,
}

fn prepare_line_gizmo_bind_group(
    mut commands: Commands,
    line_gizmo_uniform_layout: Res<LineGizmoUniformBindgroupLayout>,
    render_device: Res<RenderDevice>,
    line_gizmo_uniforms: Res<ComponentUniforms<LineGizmoUniform>>,
) {
    if let Some(binding) = line_gizmo_uniforms.uniforms().binding() {
        commands.insert_resource(LineGizmoUniformBindgroup {
            bindgroup: render_device.create_bind_group(
                "LineGizmoUniform bindgroup",
                &line_gizmo_uniform_layout.layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

struct SetLineGizmoBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetLineGizmoBindGroup<I> {
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<LineGizmoUniform>>;
    type Param = SRes<LineGizmoUniformBindgroup>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        uniform_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            &bind_group.into_inner().bindgroup,
            &[uniform_index.index()],
        );
        RenderCommandResult::Success
    }
}

struct DrawLineGizmo;
impl<P: PhaseItem> RenderCommand<P> for DrawLineGizmo {
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<LineGizmo>>;
    type Param = SRes<RenderAssets<LineGizmo>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        handle: ROQueryItem<'w, Self::ItemWorldQuery>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(line_gizmo) = line_gizmos.into_inner().get(handle) else {
            return RenderCommandResult::Failure;
        };

        if line_gizmo.vertex_count < 2 {
            return RenderCommandResult::Success;
        }

        let instances = if line_gizmo.strip {
            let item_size = VertexFormat::Float32x3.size();
            let buffer_size = line_gizmo.position_buffer.size() - item_size;
            pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(1, line_gizmo.position_buffer.slice(item_size..));

            let item_size = VertexFormat::Float32x4.size();
            let buffer_size = line_gizmo.color_buffer.size() - item_size;
            pass.set_vertex_buffer(2, line_gizmo.color_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(3, line_gizmo.color_buffer.slice(item_size..));

            u32::max(line_gizmo.vertex_count, 1) - 1
        } else {
            pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..));
            pass.set_vertex_buffer(1, line_gizmo.color_buffer.slice(..));

            line_gizmo.vertex_count / 2
        };

        pass.draw(0..6, 0..instances);

        RenderCommandResult::Success
    }
}

fn line_gizmo_vertex_buffer_layouts(strip: bool) -> Vec<VertexBufferLayout> {
    use VertexFormat::*;
    let mut position_layout = VertexBufferLayout {
        array_stride: Float32x3.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x3,
            offset: 0,
            shader_location: 0,
        }],
    };

    let mut color_layout = VertexBufferLayout {
        array_stride: Float32x4.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x4,
            offset: 0,
            shader_location: 2,
        }],
    };

    if strip {
        vec![
            position_layout.clone(),
            {
                position_layout.attributes[0].shader_location = 1;
                position_layout
            },
            color_layout.clone(),
            {
                color_layout.attributes[0].shader_location = 3;
                color_layout
            },
        ]
    } else {
        position_layout.array_stride *= 2;
        position_layout.attributes.push(VertexAttribute {
            format: Float32x3,
            offset: Float32x3.size(),
            shader_location: 1,
        });

        color_layout.array_stride *= 2;
        color_layout.attributes.push(VertexAttribute {
            format: Float32x4,
            offset: Float32x4.size(),
            shader_location: 3,
        });

        vec![position_layout, color_layout]
    }
}
