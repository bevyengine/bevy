use std::{marker::PhantomData, mem};

use bevy_app::{CoreStage, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    system::{Commands, Query, Res, ResMut, Resource, SystemParam},
};
use bevy_math::{Vec2, Vec3};
use bevy_reflect::TypeUuid;
use bevy_render::{
    prelude::{Color, Mesh, SpatialBundle},
    render_phase::AddRenderCommand,
    render_resource::{PrimitiveTopology, Shader, SpecializedMeshPipelines},
    Extract, RenderApp, RenderStage,
};

#[cfg(feature = "3d")]
use bevy_pbr::{NotShadowCaster, NotShadowReceiver};
#[cfg(feature = "2d")]
use bevy_sprite::Mesh2dHandle;

#[cfg(feature = "2d")]
pub mod pipeline_2d;
#[cfg(feature = "3d")]
pub mod pipeline_3d;

/// The `bevy_debug_draw` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{DebugDraw, DebugDraw2d, DebugDrawConfig, DebugDrawPlugin};
}

pub const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

pub struct DebugDrawPlugin;

impl Plugin for DebugDrawPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<DebugDrawResource>()
            .init_resource::<DebugDrawConfig>()
            .add_system_to_stage(CoreStage::PostUpdate, update)
            .sub_app_mut(RenderApp)
            .add_system_to_stage(RenderStage::Extract, extract);

        #[cfg(feature = "2d")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            app.sub_app_mut(RenderApp)
                .add_render_command::<Transparent2d, DrawDebugLines>()
                .init_resource::<DebugLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<DebugLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }

        #[cfg(feature = "3d")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            app.sub_app_mut(RenderApp)
                .add_render_command::<Opaque3d, DrawDebugLines>()
                .init_resource::<DebugLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<DebugLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }

        load_internal_asset!(app, SHADER_HANDLE, "debuglines.wgsl", Shader::from_wgsl);
    }
}

#[derive(Resource, Clone, Copy)]
pub struct DebugDrawConfig {
    /// Whether debug drawing should ignore depth and draw on top of everything else.
    ///
    /// Defaults to `true`.
    pub always_on_top: bool,
}

impl Default for DebugDrawConfig {
    fn default() -> Self {
        Self {
            always_on_top: true,
        }
    }
}

#[derive(Resource, Default)]
pub struct DebugDrawResource {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    mesh_handle: Option<Handle<Mesh>>,
}

impl DebugDrawResource {
    /// Draw a line from `start` to `end`.
    fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.positions
            .extend_from_slice(&[start.to_array(), end.to_array()]);
        let color = color.as_linear_rgba_f32();
        self.colors.extend_from_slice(&[color, color]);
    }

    /// Draw a line from `start` to `start + vector`.
    fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        self.line(start, start + vector, color);
    }

    fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
    }

    fn update_mesh(&mut self, mesh: &mut Mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mem::take(&mut self.positions));
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, mem::take(&mut self.colors));
    }
}

#[derive(SystemParam)]
pub struct DebugDraw<'w, 's> {
    debug_draw: ResMut<'w, DebugDrawResource>,
    #[system_param(ignore)]
    marker: PhantomData<&'s ()>,
}

impl<'w, 's> DebugDraw<'w, 's> {
    /// Draw a line from `start` to `end`.
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.debug_draw.line(start, end, color);
    }

    /// Draw a line from `start` to `start + vector`.
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        self.debug_draw.ray(start, vector, color);
    }

    pub fn clear(&mut self) {
        self.debug_draw.clear();
    }
}

#[derive(SystemParam)]
pub struct DebugDraw2d<'w, 's> {
    debug_draw: ResMut<'w, DebugDrawResource>,
    #[system_param(ignore)]
    marker: PhantomData<&'s ()>,
}

impl<'w, 's> DebugDraw2d<'w, 's> {
    /// Draw a line from `start` to `end`.
    pub fn line(&mut self, start: Vec2, end: Vec2, color: Color) {
        self.debug_draw
            .line(start.extend(0.), end.extend(0.), color);
    }

    /// Draw a line from `start` to `start + vector`.
    pub fn ray(&mut self, start: Vec2, vector: Vec2, color: Color) {
        self.debug_draw
            .ray(start.extend(0.), vector.extend(0.), color);
    }

    pub fn clear(&mut self) {
        self.debug_draw.clear();
    }
}

#[derive(Component)]
pub struct DebugDrawMesh;

pub(crate) fn update(
    mut draw: ResMut<DebugDrawResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    if let Some(mut mesh) = draw
        .mesh_handle
        .as_ref()
        .and_then(|handle| meshes.get_mut(handle))
    {
        draw.update_mesh(&mut mesh);
    } else {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        draw.update_mesh(&mut mesh);
        let mesh_handle = meshes.add(mesh);
        commands.spawn((
            SpatialBundle::VISIBLE_IDENTITY,
            DebugDrawMesh,
            #[cfg(feature = "3d")]
            (mesh_handle.clone_weak(), NotShadowCaster, NotShadowReceiver),
            #[cfg(feature = "2d")]
            Mesh2dHandle(mesh_handle.clone_weak()),
        ));
        draw.mesh_handle = Some(mesh_handle);
    }
}

/// Move the DebugDrawMesh marker Component to the render context.
pub(crate) fn extract(
    mut commands: Commands,
    query: Extract<Query<Entity, With<DebugDrawMesh>>>,
    config: Extract<Res<DebugDrawConfig>>,
) {
    for entity in &query {
        commands.get_or_spawn(entity).insert(DebugDrawMesh);
    }

    if config.is_changed() {
        commands.insert_resource(**config);
    }
}
