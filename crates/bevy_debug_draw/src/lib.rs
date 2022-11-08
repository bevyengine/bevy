use std::f32::consts::TAU;

use bevy_app::{CoreStage, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_math::{vec3, Quat, Vec2, Vec3};
use bevy_reflect::TypeUuid;
use bevy_render::{
    prelude::{Color, Mesh, SpatialBundle},
    render_phase::AddRenderCommand,
    render_resource::{PrimitiveTopology, Shader, SpecializedMeshPipelines},
    Extract, RenderApp, RenderStage,
};

#[cfg(feature = "bevy_pbr")]
use bevy_pbr::{NotShadowCaster, NotShadowReceiver};
#[cfg(feature = "bevy_sprite")]
use bevy_sprite::Mesh2dHandle;

#[cfg(feature = "bevy_sprite")]
pub mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
pub mod pipeline_3d;

/// The `bevy_debug_draw` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{DebugDraw, DebugDrawConfig, DebugDrawPlugin};
}

pub const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

pub struct DebugDrawPlugin;

impl Plugin for DebugDrawPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, SHADER_HANDLE, "debuglines.wgsl", Shader::from_wgsl);

        app.init_resource::<DebugDraw>()
            .init_resource::<DebugDrawConfig>()
            .add_system_to_stage(CoreStage::PostUpdate, update)
            .sub_app_mut(RenderApp)
            .add_system_to_stage(RenderStage::Extract, extract);

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy_core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            app.sub_app_mut(RenderApp)
                .add_render_command::<Transparent2d, DrawDebugLines>()
                .init_resource::<DebugLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<DebugLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy_core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            app.sub_app_mut(RenderApp)
                .add_render_command::<Opaque3d, DrawDebugLines>()
                .init_resource::<DebugLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<DebugLinePipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue);
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct DebugDrawConfig {
    /// Whether debug drawing should be shown.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Whether debug drawing should ignore depth and draw on top of everything else.
    ///
    /// Defaults to `true`.
    pub always_on_top: bool,
}

impl Default for DebugDrawConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            always_on_top: true,
        }
    }
}

#[derive(Resource)]
pub struct DebugDraw {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    mesh_handle: Option<Handle<Mesh>>,
    /// The amount of line segments to use when drawing a circle.
    ///
    /// Defaults to `24`.
    pub circle_segments: u32,
}

impl Default for DebugDraw {
    fn default() -> Self {
        DebugDraw {
            positions: Vec::new(),
            colors: Vec::new(),
            mesh_handle: None,
            circle_segments: 24,
        }
    }
}

impl DebugDraw {
    /// Draw a line from `start` to `end`.
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.line_gradient(start, end, color, color);
    }

    /// Draw a line from `start` to `end`.
    pub fn line_gradient(&mut self, start: Vec3, end: Vec3, start_color: Color, end_color: Color) {
        self.positions.extend([start.to_array(), end.to_array()]);
        self.colors.extend([
            start_color.as_linear_rgba_f32(),
            end_color.as_linear_rgba_f32(),
        ]);
    }

    /// Draw a line from `start` to `start + vector`.
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        self.ray_gradient(start, vector, color, color);
    }

    /// Draw a line from `start` to `start + vector`.
    pub fn ray_gradient(
        &mut self,
        start: Vec3,
        vector: Vec3,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient(start, start + vector, start_color, end_color);
    }

    /// Draw a circle at `position` with the flat side facing `normal`.
    pub fn circle(&mut self, position: Vec3, normal: Vec3, radius: f32, color: Color) {
        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        self.positions
            .extend((0..self.circle_segments).into_iter().flat_map(|i| {
                let mut angle = i as f32 * TAU / self.circle_segments as f32;
                let start = rotation * (Vec2::from(angle.sin_cos()) * radius).extend(0.) + position;

                angle += TAU / self.circle_segments as f32;
                let end = rotation * (Vec2::from(angle.sin_cos()) * radius).extend(0.) + position;

                [start.to_array(), end.to_array()]
            }));

        self.colors.extend(
            std::iter::repeat(color.as_linear_rgba_f32()).take(self.circle_segments as usize * 2),
        );
    }

    /// Draw a sphere.
    pub fn sphere(&mut self, position: Vec3, radius: f32, color: Color) {
        self.circle(position, Vec3::X, radius, color);
        self.circle(position, Vec3::Y, radius, color);
        self.circle(position, Vec3::Z, radius, color);
    }

    /// Draw a rectangle.
    pub fn rect(&mut self, position: Vec3, rotation: Quat, size: Vec2, color: Color) {
        let half_size = size / 2.;
        let tl = (position + rotation * vec3(-half_size.x, half_size.y, 0.)).to_array();
        let tr = (position + rotation * vec3(half_size.x, half_size.y, 0.)).to_array();
        let bl = (position + rotation * vec3(-half_size.x, -half_size.y, 0.)).to_array();
        let br = (position + rotation * vec3(half_size.x, -half_size.y, 0.)).to_array();
        self.positions.extend([tl, tr, tr, br, br, bl, bl, tl]);
        self.colors
            .extend(std::iter::repeat(color.as_linear_rgba_f32()).take(8))
    }

    /// Draw a box.
    pub fn cuboid(&mut self, position: Vec3, rotation: Quat, size: Vec3, color: Color) {
        let half_size = size / 2.;
        // Front
        let tlf = (position + rotation * vec3(-half_size.x, half_size.y, half_size.z)).to_array();
        let trf = (position + rotation * vec3(half_size.x, half_size.y, half_size.z)).to_array();
        let blf = (position + rotation * vec3(-half_size.x, -half_size.y, half_size.z)).to_array();
        let brf = (position + rotation * vec3(half_size.x, -half_size.y, half_size.z)).to_array();
        // Back
        let tlb = (position + rotation * vec3(-half_size.x, half_size.y, -half_size.z)).to_array();
        let trb = (position + rotation * vec3(half_size.x, half_size.y, -half_size.z)).to_array();
        let blb = (position + rotation * vec3(-half_size.x, -half_size.y, -half_size.z)).to_array();
        let brb = (position + rotation * vec3(half_size.x, -half_size.y, -half_size.z)).to_array();
        self.positions.extend([
            tlf, trf, trf, brf, brf, blf, blf, tlf, // Front
            tlb, trb, trb, brb, brb, blb, blb, tlb, // Back
            tlf, tlb, trf, trb, brf, brb, blf, blb, // Front to back
        ]);
        self.colors
            .extend(std::iter::repeat(color.as_linear_rgba_f32()).take(24))
    }

    /// Draw a line from `start` to `end`.
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: Color) {
        self.line_gradient_2d(start, end, color, color);
    }

    /// Draw a line from `start` to `end`.
    pub fn line_gradient_2d(
        &mut self,
        start: Vec2,
        end: Vec2,
        start_color: Color,
        end_color: Color,
    ) {
        self.line_gradient(start.extend(0.), end.extend(0.), start_color, end_color);
    }

    /// Draw a line from `start` to `start + vector`.
    pub fn ray_2d(&mut self, start: Vec2, vector: Vec2, color: Color) {
        self.ray(start.extend(0.), vector.extend(0.), color);
    }

    // Draw a circle.
    pub fn circle_2d(&mut self, position: Vec2, radius: f32, color: Color) {
        self.circle(position.extend(0.), Vec3::Z, radius, color);
    }

    /// Draw a rectangle.
    pub fn rect_2d(&mut self, position: Vec2, rotation: f32, size: Vec2, color: Color) {
        self.rect(
            position.extend(0.),
            Quat::from_rotation_z(rotation),
            size,
            color,
        );
    }

    /// Clear everything drawn up to this point, this frame.
    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
    }

    /// Take the positions and colors data from `self` and overwrite the `mesh`'s vertex positions and colors.
    pub fn update_mesh(&mut self, mesh: &mut Mesh) {
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            std::mem::take(&mut self.positions),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, std::mem::take(&mut self.colors));
    }
}

#[derive(Component)]
pub struct DebugDrawMesh;

pub(crate) fn update(
    config: Res<DebugDrawConfig>,
    mut debug_draw: ResMut<DebugDraw>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    if let Some(mut mesh) = debug_draw
        .mesh_handle
        .as_ref()
        .and_then(|handle| meshes.get_mut(handle))
    {
        if config.enabled {
            debug_draw.update_mesh(&mut mesh);
        } else {
            debug_draw.clear();
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION);
            mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
        }
    } else if config.enabled {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        debug_draw.update_mesh(&mut mesh);
        let mesh_handle = meshes.add(mesh);
        commands.spawn((
            SpatialBundle::VISIBLE_IDENTITY,
            DebugDrawMesh,
            #[cfg(feature = "bevy_pbr")]
            (mesh_handle.clone_weak(), NotShadowCaster, NotShadowReceiver),
            #[cfg(feature = "bevy_sprite")]
            Mesh2dHandle(mesh_handle.clone_weak()),
        ));
        debug_draw.mesh_handle = Some(mesh_handle);
    } else {
        debug_draw.clear();
    }
}

/// Move the DebugDrawMesh marker Component and the DebugDrawConfig Resource to the render context.
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
