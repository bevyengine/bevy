use bevy_app::{CoreStage, Plugin};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    prelude::Component,
    query::{With, Without},
    schedule::ParallelSystemDescriptorCoercion,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_math::{Affine3A, Vec3};
use bevy_render::{
    prelude::{Color, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_transform::{prelude::GlobalTransform, TransformSystem};
use bevy_utils::default;

use crate::{NotShadowCaster, NotShadowReceiver, PbrBundle, StandardMaterial};

pub struct DebugLinesPlugin;

impl Plugin for DebugLinesPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<DebugLines>()
            .add_startup_system(init_debug_lines)
            .add_system_to_stage(CoreStage::PreUpdate, reset_debug_lines)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_debug_lines_entity.after(draw_debug_normals),
            );
    }
}

#[derive(Resource)]
pub struct DebugLines {
    color: Color,
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    mesh_handle: Option<Handle<Mesh>>,
}

impl DebugLines {
    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn draw_line(&mut self, start: Vec3, end: Vec3) {
        self.positions.push(start.to_array());
        self.positions.push(end.to_array());
        let color = self.color.as_linear_rgba_f32();
        self.colors.push(color);
        self.colors.push(color);
    }

    pub fn positions(&self) -> &Vec<[f32; 3]> {
        &self.positions
    }

    pub fn colors(&self) -> &Vec<[f32; 4]> {
        &self.colors
    }

    pub fn update_mesh(&mut self, mesh: &mut Mesh) {
        let n_vertices = self.positions.len();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            std::mem::take(&mut self.positions),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32; 3]; n_vertices]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, std::mem::take(&mut self.colors));
    }
}

impl Default for DebugLines {
    fn default() -> Self {
        Self {
            color: Color::GREEN,
            positions: Vec::new(),
            colors: Vec::new(),
            mesh_handle: None,
        }
    }
}

#[derive(Component)]
struct DebugLinesMesh;

fn init_debug_lines(
    mut commands: Commands,
    mut debug_lines: ResMut<DebugLines>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Mesh::new(PrimitiveTopology::LineList));
    commands
        .spawn_bundle(PbrBundle {
            mesh: mesh.clone_weak(),
            material: materials.add(StandardMaterial {
                unlit: true,
                ..default()
            }),
            ..default()
        })
        .insert_bundle((NotShadowCaster, NotShadowReceiver, DebugLinesMesh));
    debug_lines.mesh_handle = Some(mesh);
}

fn reset_debug_lines(mut debug_lines: ResMut<DebugLines>) {
    debug_lines.clear();
}

fn update_debug_lines_entity(
    mut debug_lines: ResMut<DebugLines>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mesh_handle = debug_lines.mesh_handle.as_ref().unwrap();
    if let Some(mesh) = meshes.get_mut(mesh_handle) {
        debug_lines.update_mesh(mesh);
    }
}

pub struct DebugNormalsPlugin;

impl Plugin for DebugNormalsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        if app.world.get_resource::<DebugNormalsSettings>().is_none() {
            app.init_resource::<DebugNormalsSettings>();
        }
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            draw_debug_normals.after(TransformSystem::TransformPropagate),
        );
    }
}

#[derive(Resource)]
pub struct DebugNormalsSettings {
    pub global: bool,
    pub color: Color,
    pub scale: f32,
}

impl Default for DebugNormalsSettings {
    fn default() -> Self {
        Self {
            global: false,
            color: Color::GREEN,
            scale: 0.01,
        }
    }
}

#[derive(Component)]
pub struct DebugNormals;

fn draw_debug_normals(
    debug_normal_settings: Res<DebugNormalsSettings>,
    debug_normals: Query<
        (&GlobalTransform, &Handle<Mesh>),
        (With<DebugNormals>, Without<DebugLinesMesh>),
    >,
    no_debug_normals: Query<
        (&GlobalTransform, &Handle<Mesh>),
        (Without<DebugNormals>, Without<DebugLinesMesh>),
    >,
    meshes: Res<Assets<Mesh>>,
    debug_lines: ResMut<DebugLines>,
) {
    let debug_lines = debug_lines.into_inner();
    let scale = debug_normal_settings.scale;

    debug_lines.set_color(debug_normal_settings.color);

    for (transform, mesh) in &debug_normals {
        if let Some(mesh) = meshes.get(mesh) {
            draw_mesh_normals(&transform.affine(), mesh, scale, debug_lines);
        }
    }
    if debug_normal_settings.global {
        for (transform, mesh) in &no_debug_normals {
            if let Some(mesh) = meshes.get(mesh) {
                draw_mesh_normals(&transform.affine(), mesh, scale, debug_lines);
            }
        }
    }
}

fn draw_mesh_normals(
    transform: &Affine3A,
    reference_mesh: &Mesh,
    scale: f32,
    debug_lines: &mut DebugLines,
) {
    let inv_transpose = transform.matrix3.inverse().transpose();
    // For each vertex, create a line from the vertex position in the direction of the normal
    let ref_positions = reference_mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let ref_normals = reference_mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap();
    for (position, normal) in ref_positions.iter().zip(ref_normals.iter()) {
        let position = transform.transform_point3(Vec3::from_slice(position));
        let normal = inv_transpose * Vec3::from_slice(normal);
        debug_lines.draw_line(position, position + scale * normal);
    }
}
