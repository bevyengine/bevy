mod gpu;

use super::{Indices, Mesh};
use crate::{renderer::RenderDevice, settings::WgpuFeatures, RenderApp};
use bevy_app::{App, Plugin};
use bevy_asset::{
    anyhow::{bail, Error},
    Asset, AssetApp,
};
use bevy_ecs::system::Resource;
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use meshopt::{build_meshlets, compute_meshlet_bounds_decoder, VertexDataAdapter};
use serde::{Deserialize, Serialize};
use wgpu::PrimitiveTopology;

pub struct MeshletPlugin;

impl Plugin for MeshletPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MeshletMesh>();
    }

    fn finish(&self, app: &mut App) {
        let required_features = WgpuFeatures::MULTI_DRAW_INDIRECT;
        match app.world.get_resource::<RenderDevice>() {
            Some(render_device) if render_device.features().contains(required_features) => {}
            _ => return,
        }

        app.insert_resource(MeshletRenderingSupported);

        app.sub_app_mut(RenderApp)
            .insert_resource(MeshletRenderingSupported);
    }
}

#[derive(Resource)]
pub struct MeshletRenderingSupported;

#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct MeshletMesh {
    pub mesh_vertex_data: Box<[u8]>,
    pub meshlet_vertex_buffer: Box<[u32]>,
    pub meshlet_index_buffer: Box<[u8]>,
    pub meshlets: Box<[Meshlet]>,
    pub meshlet_bounding_spheres: Box<[MeshletBoundingSphere]>,
    pub meshlet_bounding_cones: Box<[MeshletBoundingCone]>,
}

#[derive(Serialize, Deserialize)]
pub struct Meshlet {
    pub meshlet_vertex_buffer_index: u32,
    pub meshlet_index_buffer_index: u32,
    pub meshlet_vertex_count: u32,
    pub meshlet_triangle_count: u32,
}

#[derive(Serialize, Deserialize)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Serialize, Deserialize)]
pub struct MeshletBoundingCone {
    pub apex: Vec3,
    pub axis: Vec3,
}

impl MeshletMesh {
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, Error> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            bail!("Mesh primitive_topology was not TriangleList");
        }
        let vertex_buffer_layout = &mesh.get_mesh_vertex_buffer_layout();
        if vertex_buffer_layout.attribute_ids()
            != &[
                Mesh::ATTRIBUTE_POSITION.id,
                Mesh::ATTRIBUTE_NORMAL.id,
                Mesh::ATTRIBUTE_UV_0.id,
                Mesh::ATTRIBUTE_TANGENT.id,
            ]
        {
            bail!("Mesh attributes were not [POSITION, NORMAL, UV_0, TANGENT]");
        }
        let indices = match mesh.indices() {
            Some(Indices::U32(indices)) => indices,
            _ => bail!("Mesh indicies were not Some(Indices::U32)"),
        };
        let vertex_buffer = mesh.get_vertex_buffer_data();
        let vertices = VertexDataAdapter::new(
            &vertex_buffer,
            vertex_buffer_layout.layout().array_stride as usize,
            0,
        )?;

        let meshlets = build_meshlets(indices, &vertices, 126, 64, 0.5);

        let mut meshlet_bounding_spheres = Vec::with_capacity(meshlets.meshlets.len());
        let mut meshlet_bounding_cones = Vec::with_capacity(meshlets.meshlets.len());
        for meshlet in meshlets.iter() {
            let bounds = compute_meshlet_bounds_decoder(
                meshlet,
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    .unwrap()
                    .as_float3()
                    .unwrap(),
            );
            meshlet_bounding_spheres.push(MeshletBoundingSphere {
                center: bounds.center.into(),
                radius: bounds.radius,
            });
            meshlet_bounding_cones.push(MeshletBoundingCone {
                apex: bounds.cone_apex.into(),
                axis: bounds.cone_axis.into(),
            });
        }

        Ok(Self {
            mesh_vertex_data: vertex_buffer.into_boxed_slice(),
            meshlet_vertex_buffer: meshlets.vertices.into_boxed_slice(),
            meshlet_index_buffer: meshlets.triangles.into_boxed_slice(),
            meshlets: meshlets
                .meshlets
                .into_iter()
                .map(|m| Meshlet {
                    meshlet_vertex_buffer_index: m.vertex_offset,
                    meshlet_index_buffer_index: m.triangle_offset,
                    meshlet_vertex_count: m.vertex_count,
                    meshlet_triangle_count: m.triangle_count,
                })
                .collect(),
            meshlet_bounding_spheres: meshlet_bounding_spheres.into_boxed_slice(),
            meshlet_bounding_cones: meshlet_bounding_cones.into_boxed_slice(),
        })
    }
}
