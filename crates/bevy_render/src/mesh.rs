use crate::{
    pipeline::{
        state_descriptors::{IndexFormat, PrimitiveTopology},
        AsVertexBufferDescriptor, VertexBufferDescriptor, VertexBufferDescriptors, VertexFormat,
    },
    render_resource::{BufferInfo, BufferUsage, RenderResourceId},
    renderer::RenderResourceContext,
    RenderPipelines, Vertex,
};
use bevy_app::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::bytes::AsBytes;
use glam::*;
use legion::prelude::*;
use std::{borrow::Cow, collections::HashSet};
use thiserror::Error;

pub const VERTEX_BUFFER_ASSET_INDEX: usize = 0;
pub const INDEX_BUFFER_ASSET_INDEX: usize = 1;
#[derive(Clone, Debug)]
pub enum VertexAttributeValues {
    Float(Vec<f32>),
    Float2(Vec<[f32; 2]>),
    Float3(Vec<[f32; 3]>),
    Float4(Vec<[f32; 4]>),
}

impl VertexAttributeValues {
    pub fn len(&self) -> usize {
        match *self {
            VertexAttributeValues::Float(ref values) => values.len(),
            VertexAttributeValues::Float2(ref values) => values.len(),
            VertexAttributeValues::Float3(ref values) => values.len(),
            VertexAttributeValues::Float4(ref values) => values.len(),
        }
    }

    // TODO: add vertex format as parameter here and perform type conversions
    pub fn get_bytes(&self) -> &[u8] {
        match self {
            VertexAttributeValues::Float(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Float2(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Float3(values) => values.as_slice().as_bytes(),
            VertexAttributeValues::Float4(values) => values.as_slice().as_bytes(),
        }
    }
}

impl From<&VertexAttributeValues> for VertexFormat {
    fn from(values: &VertexAttributeValues) -> Self {
        match values {
            VertexAttributeValues::Float(_) => VertexFormat::Float,
            VertexAttributeValues::Float2(_) => VertexFormat::Float2,
            VertexAttributeValues::Float3(_) => VertexFormat::Float3,
            VertexAttributeValues::Float4(_) => VertexFormat::Float4,
        }
    }
}

#[derive(Debug)]
pub struct VertexAttribute {
    pub name: Cow<'static, str>,
    pub values: VertexAttributeValues,
}

impl VertexAttribute {
    pub const POSITION: &'static str = "Vertex_Position";
    pub const NORMAL: &'static str = "Vertex_Normal";
    pub const UV: &'static str = "Vertex_Uv";

    pub fn position(positions: Vec<[f32; 3]>) -> Self {
        VertexAttribute {
            name: Self::POSITION.into(),
            values: VertexAttributeValues::Float3(positions),
        }
    }

    pub fn normal(normals: Vec<[f32; 3]>) -> Self {
        VertexAttribute {
            name: Self::NORMAL.into(),
            values: VertexAttributeValues::Float3(normals),
        }
    }

    pub fn uv(uvs: Vec<[f32; 2]>) -> Self {
        VertexAttribute {
            name: Self::UV.into(),
            values: VertexAttributeValues::Float2(uvs),
        }
    }
}

#[derive(Error, Debug)]
pub enum MeshToVertexBufferError {
    #[error("VertexBufferDescriptor requires a VertexBufferAttribute this Mesh does not contain.")]
    MissingVertexAttribute { attribute_name: Cow<'static, str> },
    #[error("Mesh VertexAttribute VertexFormat is incompatible with VertexBufferDescriptor VertexAttribute VertexFormat.")]
    IncompatibleVertexAttributeFormat {
        attribute_name: Cow<'static, str>,
        descriptor_format: VertexFormat,
        mesh_format: VertexFormat,
    },
}

#[derive(Debug)]
pub struct Mesh {
    pub primitive_topology: PrimitiveTopology,
    pub attributes: Vec<VertexAttribute>,
    pub indices: Option<Vec<u32>>,
}

impl Mesh {
    pub fn new(primitive_topology: PrimitiveTopology) -> Self {
        Mesh {
            primitive_topology,
            attributes: Vec::new(),
            indices: None,
        }
    }

    pub fn get_vertex_buffer_bytes(
        &self,
        vertex_buffer_descriptor: &VertexBufferDescriptor,
    ) -> Result<Vec<u8>, MeshToVertexBufferError> {
        let length = self.attributes.first().map(|a| a.values.len()).unwrap_or(0);
        let mut bytes = vec![0; vertex_buffer_descriptor.stride as usize * length];

        for vertex_attribute in vertex_buffer_descriptor.attributes.iter() {
            match self
                .attributes
                .iter()
                .find(|a| vertex_attribute.name == a.name)
            {
                Some(mesh_attribute) => {
                    let attribute_bytes = mesh_attribute.values.get_bytes();
                    let attribute_size = vertex_attribute.format.get_size() as usize;
                    for (i, vertex_slice) in attribute_bytes.chunks(attribute_size).enumerate() {
                        let vertex_offset = vertex_buffer_descriptor.stride as usize * i;
                        let attribute_offset = vertex_offset + vertex_attribute.offset as usize;
                        bytes[attribute_offset..attribute_offset + attribute_size]
                            .copy_from_slice(vertex_slice);
                    }
                }
                None => {
                    return Err(MeshToVertexBufferError::MissingVertexAttribute {
                        attribute_name: vertex_attribute.name.clone(),
                    })
                }
            }
        }

        Ok(bytes)
    }

    pub fn get_index_buffer_bytes(&self, index_format: IndexFormat) -> Option<Vec<u8>> {
        self.indices.as_ref().map(|indices| match index_format {
            IndexFormat::Uint16 => indices
                .iter()
                .map(|i| *i as u16)
                .collect::<Vec<u16>>()
                .as_slice()
                .as_bytes()
                .to_vec(),
            IndexFormat::Uint32 => indices.as_slice().as_bytes().to_vec(),
        })
    }
}

pub mod shape {
    use super::{Mesh, VertexAttribute};
    use crate::pipeline::state_descriptors::PrimitiveTopology;
    use glam::*;

    pub struct Cube {
        pub size: f32,
    }

    impl Default for Cube {
        fn default() -> Self {
            Cube { size: 1.0 }
        }
    }

    impl From<Cube> for Mesh {
        fn from(cube: Cube) -> Self {
            let size = cube.size;
            let vertices = &[
                // top (0., 0., size)
                ([-size, -size, size], [0., 0., size], [0., 0.]),
                ([size, -size, size], [0., 0., size], [size, 0.]),
                ([size, size, size], [0., 0., size], [size, size]),
                ([-size, size, size], [0., 0., size], [0., size]),
                // bottom (0., 0., -size)
                ([-size, size, -size], [0., 0., -size], [size, 0.]),
                ([size, size, -size], [0., 0., -size], [0., 0.]),
                ([size, -size, -size], [0., 0., -size], [0., size]),
                ([-size, -size, -size], [0., 0., -size], [size, size]),
                // right (size, 0., 0.)
                ([size, -size, -size], [size, 0., 0.], [0., 0.]),
                ([size, size, -size], [size, 0., 0.], [size, 0.]),
                ([size, size, size], [size, 0., 0.], [size, size]),
                ([size, -size, size], [size, 0., 0.], [0., size]),
                // left (-size, 0., 0.)
                ([-size, -size, size], [-size, 0., 0.], [size, 0.]),
                ([-size, size, size], [-size, 0., 0.], [0., 0.]),
                ([-size, size, -size], [-size, 0., 0.], [0., size]),
                ([-size, -size, -size], [-size, 0., 0.], [size, size]),
                // front (0., size, 0.)
                ([size, size, -size], [0., size, 0.], [size, 0.]),
                ([-size, size, -size], [0., size, 0.], [0., 0.]),
                ([-size, size, size], [0., size, 0.], [0., size]),
                ([size, size, size], [0., size, 0.], [size, size]),
                // back (0., -size, 0.)
                ([size, -size, size], [0., -size, 0.], [0., 0.]),
                ([-size, -size, size], [0., -size, 0.], [size, 0.]),
                ([-size, -size, -size], [0., -size, 0.], [size, size]),
                ([size, -size, -size], [0., -size, 0.], [0., size]),
            ];

            let mut positions = Vec::new();
            let mut normals = Vec::new();
            let mut uvs = Vec::new();
            for (position, normal, uv) in vertices.iter() {
                positions.push(position.clone());
                normals.push(normal.clone());
                uvs.push(uv.clone());
            }

            let indices = vec![
                0, 1, 2, 2, 3, 0, // top
                4, 5, 6, 6, 7, 4, // bottom
                8, 9, 10, 10, 11, 8, // right
                12, 13, 14, 14, 15, 12, // left
                16, 17, 18, 18, 19, 16, // front
                20, 21, 22, 22, 23, 20, // back
            ];

            Mesh {
                primitive_topology: PrimitiveTopology::TriangleList,
                attributes: vec![
                    VertexAttribute::position(positions),
                    VertexAttribute::normal(normals),
                    VertexAttribute::uv(uvs),
                ],
                indices: Some(indices),
            }
        }
    }

    pub struct Quad {
        pub size: Vec2,
    }

    impl From<Quad> for Mesh {
        fn from(quad: Quad) -> Self {
            let extent_x = quad.size.x() / 2.0;
            let extent_y = quad.size.y() / 2.0;

            let north_west = vec2(-extent_x, extent_y);
            let north_east = vec2(extent_x, extent_y);
            let south_west = vec2(-extent_x, -extent_y);
            let south_east = vec2(extent_x, -extent_y);
            let vertices = &[
                (
                    [south_west.x(), south_west.y(), 0.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 1.0],
                ),
                (
                    [north_west.x(), north_west.y(), 0.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0],
                ),
                (
                    [north_east.x(), north_east.y(), 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 0.0],
                ),
                (
                    [south_east.x(), south_east.y(), 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 1.0],
                ),
            ];

            let indices = vec![0, 2, 1, 0, 3, 2];

            let mut positions = Vec::new();
            let mut normals = Vec::new();
            let mut uvs = Vec::new();
            for (position, normal, uv) in vertices.iter() {
                positions.push(position.clone());
                normals.push(normal.clone());
                uvs.push(uv.clone());
            }

            Mesh {
                primitive_topology: PrimitiveTopology::TriangleList,
                attributes: vec![
                    VertexAttribute::position(positions),
                    VertexAttribute::normal(normals),
                    VertexAttribute::uv(uvs),
                ],
                indices: Some(indices),
            }
        }
    }

    pub struct Plane {
        pub size: f32,
    }

    impl From<Plane> for Mesh {
        fn from(plane: Plane) -> Self {
            Quad {
                size: Vec2::new(plane.size, plane.size),
            }
            .into()
        }
    }
}

fn remove_current_mesh_resources(
    render_resource_context: &dyn RenderResourceContext,
    handle: Handle<Mesh>,
) {
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(handle, VERTEX_BUFFER_ASSET_INDEX)
    {
        render_resource_context.remove_buffer(buffer);
        render_resource_context.remove_asset_resource(handle, VERTEX_BUFFER_ASSET_INDEX);
    }
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX)
    {
        render_resource_context.remove_buffer(buffer);
        render_resource_context.remove_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX);
    }
}

pub fn mesh_resource_provider_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();
    let mut mesh_event_reader = EventReader::<AssetEvent<Mesh>>::default();
    // TODO: allow pipelines to specialize on vertex_buffer_descriptor and index_format
    let vertex_buffer_descriptor = Vertex::as_vertex_buffer_descriptor();
    vertex_buffer_descriptors.set(vertex_buffer_descriptor.clone());
    (move |world: &mut SubWorld,
           render_resource_context: Res<Box<dyn RenderResourceContext>>,
           meshes: Res<Assets<Mesh>>,
           mesh_events: Res<Events<AssetEvent<Mesh>>>,
           query: &mut Query<(Read<Handle<Mesh>>, Write<RenderPipelines>)>| {
        let mut changed_meshes = HashSet::new();
        let render_resource_context = &**render_resource_context;
        for event in mesh_event_reader.iter(&mesh_events) {
            match event {
                AssetEvent::Created { handle } => {
                    changed_meshes.insert(*handle);
                }
                AssetEvent::Modified { handle } => {
                    changed_meshes.insert(*handle);
                    remove_current_mesh_resources(render_resource_context, *handle);
                }
                AssetEvent::Removed { handle } => {
                    remove_current_mesh_resources(render_resource_context, *handle);
                    // if mesh was modified and removed in the same update, ignore the modification
                    // events are ordered so future modification events are ok
                    changed_meshes.remove(handle);
                }
            }
        }

        for changed_mesh_handle in changed_meshes.iter() {
            if let Some(mesh) = meshes.get(changed_mesh_handle) {
                let vertex_bytes = mesh
                    .get_vertex_buffer_bytes(&vertex_buffer_descriptor)
                    .unwrap();
                // TODO: use a staging buffer here
                let vertex_buffer = render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                    &vertex_bytes,
                );

                let index_bytes = mesh.get_index_buffer_bytes(IndexFormat::Uint16).unwrap();
                let index_buffer = render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::INDEX,
                        ..Default::default()
                    },
                    &index_bytes,
                );

                render_resource_context.set_asset_resource(
                    *changed_mesh_handle,
                    RenderResourceId::Buffer(vertex_buffer),
                    VERTEX_BUFFER_ASSET_INDEX,
                );
                render_resource_context.set_asset_resource(
                    *changed_mesh_handle,
                    RenderResourceId::Buffer(index_buffer),
                    INDEX_BUFFER_ASSET_INDEX,
                );
            }
        }

        // TODO: remove this once batches are pipeline specific and deprecate assigned_meshes draw target
        for (handle, mut render_pipelines) in query.iter_mut(world) {
            if let Some(mesh) = meshes.get(&handle) {
                render_pipelines
                    .render_resource_bindings
                    .pipeline_specialization
                    .primitive_topology = mesh.primitive_topology;
            }

            if let Some(RenderResourceId::Buffer(vertex_buffer)) =
                render_resource_context.get_asset_resource(*handle, VERTEX_BUFFER_ASSET_INDEX)
            {
                render_pipelines.render_resource_bindings.set_vertex_buffer(
                    "Vertex",
                    vertex_buffer,
                    render_resource_context
                        .get_asset_resource(*handle, INDEX_BUFFER_ASSET_INDEX)
                        .and_then(|r| {
                            if let RenderResourceId::Buffer(buffer) = r {
                                Some(buffer)
                            } else {
                                None
                            }
                        }),
                );
            }
        }
    })
    .system()
}

#[cfg(test)]
mod tests {
    use super::{AsVertexBufferDescriptor, Mesh, VertexAttribute};
    use crate::{pipeline::state_descriptors::PrimitiveTopology, Vertex};
    use bevy_core::bytes::AsBytes;

    #[test]
    fn test_get_vertex_bytes() {
        let vertices = &[
            ([0., 0., 0.], [1., 1., 1.], [2., 2.]),
            ([3., 3., 3.], [4., 4., 4.], [5., 5.]),
            ([6., 6., 6.], [7., 7., 7.], [8., 8.]),
        ];

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        for (position, normal, uv) in vertices.iter() {
            positions.push(*position);
            normals.push(*normal);
            uvs.push(*uv);
        }

        let mesh = Mesh {
            primitive_topology: PrimitiveTopology::TriangleStrip,
            attributes: vec![
                VertexAttribute::position(positions),
                VertexAttribute::normal(normals),
                VertexAttribute::uv(uvs),
            ],
            indices: None,
        };

        let expected_vertices = &[
            Vertex {
                position: [0., 0., 0.],
                normal: [1., 1., 1.],
                uv: [2., 2.],
            },
            Vertex {
                position: [3., 3., 3.],
                normal: [4., 4., 4.],
                uv: [5., 5.],
            },
            Vertex {
                position: [6., 6., 6.],
                normal: [7., 7., 7.],
                uv: [8., 8.],
            },
        ];

        let descriptor = Vertex::as_vertex_buffer_descriptor();
        assert_eq!(
            mesh.get_vertex_buffer_bytes(descriptor).unwrap(),
            expected_vertices.as_bytes(),
            "buffer bytes are equal"
        );
    }
}
