use crate::{
    pipeline::{PrimitiveTopology, RenderPipelines, VertexFormat},
    renderer::{BufferInfo, BufferUsage, RenderResourceContext, RenderResourceId},
};
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::AsBytes;
use bevy_ecs::{Local, Query, Res};
use bevy_math::*;
use std::borrow::Cow;
use thiserror::Error;

use bevy_utils::HashMap;
use std::hash::{Hash, Hasher};

pub const VERTEX_BUFFER_ASSET_INDEX: usize = 1;
pub const INDEX_BUFFER_ASSET_INDEX: usize = 0;
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
pub struct VertexAttributeData {
    //TODO: replace by tuple?
    pub name: Cow<'static, str>,
    pub values: VertexAttributeValues, //TODO: the values aren't necessarily needed after been submitted to the GPU
}

impl PartialEq for VertexAttributeData {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for VertexAttributeData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Eq for VertexAttributeData {}

impl VertexAttributeData {
    pub const NORMAL: &'static str = "normal_os";
    pub const POSITION: &'static str = "position_os";
    pub const UV: &'static str = "uv_vertex";

    pub fn position(positions: Vec<[f32; 3]>) -> Self {
        VertexAttributeData {
            name: Self::POSITION.into(),
            values: VertexAttributeValues::Float3(positions),
        }
    }

    pub fn normal(normals: Vec<[f32; 3]>) -> Self {
        VertexAttributeData {
            name: Self::NORMAL.into(),
            values: VertexAttributeValues::Float3(normals),
        }
    }

    pub fn uv(uvs: Vec<[f32; 2]>) -> Self {
        VertexAttributeData {
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
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

#[derive(Debug)]
pub struct Mesh {
    pub primitive_topology: PrimitiveTopology,
    attributes: HashMap<Cow<'static, str>, VertexAttributeData>,
    pub indices: Option<Indices>,
}

impl Mesh {
    pub fn new(
        primitive_topology: PrimitiveTopology,
        mut attributes: Vec<VertexAttributeData>,
        indices: Option<Indices>,
    ) -> Self {
        let mut mesh = Mesh {
            primitive_topology,
            attributes: HashMap::default(),
            indices,
        };
        for attribute in attributes.drain(..) {
            mesh.insert_attribute(attribute);
        }
        mesh
    }

    pub fn new_empty(primitive_topology: PrimitiveTopology) -> Self {
        Mesh {
            primitive_topology,
            attributes: HashMap::default(),
            indices: None,
        }
    }

    pub fn insert_attribute(&mut self, new_vertex_attribute: VertexAttributeData) {
        self.attributes
            .insert(new_vertex_attribute.name.clone(), new_vertex_attribute); //TODO: is .clone() correct?
    }

    pub fn get_attribute(&mut self, name: Cow<'static, str>) -> Option<&VertexAttributeData> {
        self.attributes.get(&name.clone()) //Todo julian: clone??
    }

    pub fn remove_attribute(&mut self, name: Cow<'static, str>) {
        self.attributes.remove(&name);
    }

    pub fn iter_attribute(
        &self,
    ) -> std::collections::hash_map::Iter<'_, Cow<'static, str>, VertexAttributeData> {
        self.attributes.iter()
    }

    pub fn get_index_buffer_bytes(&self) -> Option<Vec<u8>> {
        self.indices.as_ref().map(|indices| match &indices {
            Indices::U16(indices) => indices.as_slice().as_bytes().to_vec(),
            Indices::U32(indices) => indices.as_slice().as_bytes().to_vec(),
        })
    }
}

/// Generation for some primitive shape meshes.
pub mod shape {
    use super::{Indices, Mesh, VertexAttributeData};
    use crate::pipeline::PrimitiveTopology;
    use bevy_math::*;
    use hexasphere::Hexasphere;

    /// A cube.
    pub struct Cube {
        /// Half the side length of the cube.
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
                positions.push(*position);
                normals.push(*normal);
                uvs.push(*uv);
            }

            let indices = Indices::U32(vec![
                0, 1, 2, 2, 3, 0, // top
                4, 5, 6, 6, 7, 4, // bottom
                8, 9, 10, 10, 11, 8, // right
                12, 13, 14, 14, 15, 12, // left
                16, 17, 18, 18, 19, 16, // front
                20, 21, 22, 22, 23, 20, // back
            ]);

            Mesh::new(
                PrimitiveTopology::TriangleList,
                vec![
                    VertexAttributeData::position(positions),
                    VertexAttributeData::normal(normals),
                    VertexAttributeData::uv(uvs),
                ],
                Some(indices),
            )
        }
    }

    /// A rectangle on the XY plane.
    pub struct Quad {
        /// Full width and height of the rectangle.
        pub size: Vec2,
        /// Flips the texture coords of the resulting vertices.
        pub flip: bool,
    }

    impl Quad {
        pub fn new(size: Vec2) -> Self {
            Self { size, flip: false }
        }

        pub fn flipped(size: Vec2) -> Self {
            Self { size, flip: true }
        }
    }

    impl From<Quad> for Mesh {
        fn from(quad: Quad) -> Self {
            let extent_x = quad.size.x() / 2.0;
            let extent_y = quad.size.y() / 2.0;

            let north_west = vec2(-extent_x, extent_y);
            let north_east = vec2(extent_x, extent_y);
            let south_west = vec2(-extent_x, -extent_y);
            let south_east = vec2(extent_x, -extent_y);
            let vertices = if quad.flip {
                [
                    (
                        [south_east.x(), south_east.y(), 0.0],
                        [0.0, 0.0, 1.0],
                        [1.0, 1.0],
                    ),
                    (
                        [north_east.x(), north_east.y(), 0.0],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0],
                    ),
                    (
                        [north_west.x(), north_west.y(), 0.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0],
                    ),
                    (
                        [south_west.x(), south_west.y(), 0.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 1.0],
                    ),
                ]
            } else {
                [
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
                ]
            };

            let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

            let mut positions = Vec::new();
            let mut normals = Vec::new();
            let mut uvs = Vec::new();
            for (position, normal, uv) in vertices.iter() {
                positions.push(*position);
                normals.push(*normal);
                uvs.push(*uv);
            }

            Mesh::new(
                PrimitiveTopology::TriangleList,
                vec![
                    VertexAttributeData::position(positions),
                    VertexAttributeData::normal(normals),
                    VertexAttributeData::uv(uvs),
                ],
                Some(indices),
            )
        }
    }

    /// A square on the XZ plane.
    pub struct Plane {
        /// The total side length of the square.
        pub size: f32,
    }

    impl From<Plane> for Mesh {
        fn from(plane: Plane) -> Self {
            let extent = plane.size / 2.0;

            let vertices = [
                ([extent, 0.0, -extent], [0.0, 1.0, 0.0], [1.0, 1.0]),
                ([extent, 0.0, extent], [0.0, 1.0, 0.0], [1.0, 0.0]),
                ([-extent, 0.0, extent], [0.0, 1.0, 0.0], [0.0, 0.0]),
                ([-extent, 0.0, -extent], [0.0, 1.0, 0.0], [0.0, 1.0]),
            ];

            let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

            let mut positions = Vec::new();
            let mut normals = Vec::new();
            let mut uvs = Vec::new();
            for (position, normal, uv) in vertices.iter() {
                positions.push(*position);
                normals.push(*normal);
                uvs.push(*uv);
            }

            Mesh::new(
                PrimitiveTopology::TriangleList,
                vec![
                    VertexAttributeData::position(positions),
                    VertexAttributeData::normal(normals),
                    VertexAttributeData::uv(uvs),
                ],
                Some(indices),
            )
        }
    }

    /// A sphere made from a subdivided Icosahedron.
    pub struct Icosphere {
        /// The radius of the sphere.
        pub radius: f32,
        /// The number of subdivisions applied.
        pub subdivisions: usize,
    }

    impl Default for Icosphere {
        fn default() -> Self {
            Self {
                radius: 1.0,
                subdivisions: 5,
            }
        }
    }

    impl From<Icosphere> for Mesh {
        fn from(sphere: Icosphere) -> Self {
            if sphere.subdivisions >= 80 {
                let temp_sphere = Hexasphere::new(sphere.subdivisions, |_| ());

                panic!(
                    "Cannot create an icosphere of {} subdivisions due to there being too many vertices being generated: {} (Limited to 65535 vertices or 79 subdivisions)",
                    sphere.subdivisions,
                    temp_sphere.raw_points().len()
                );
            }
            let hexasphere = Hexasphere::new(sphere.subdivisions, |point| {
                let inclination = point.z().acos();
                let azumith = point.y().atan2(point.x());

                let norm_inclination = 1.0 - (inclination / std::f32::consts::PI);
                let norm_azumith = (azumith / std::f32::consts::PI) * 0.5;

                [norm_inclination, norm_azumith]
            });

            let raw_points = hexasphere.raw_points();

            let points = raw_points
                .iter()
                .map(|&p| (p * sphere.radius).into())
                .collect::<Vec<[f32; 3]>>();

            let normals = raw_points
                .iter()
                .copied()
                .map(Into::into)
                .collect::<Vec<[f32; 3]>>();

            let uvs = hexasphere.raw_data().to_owned();

            let mut indices = Vec::with_capacity(hexasphere.indices_per_main_triangle() * 20);

            for i in 0..20 {
                hexasphere.get_indices(i, &mut indices);
            }

            let indices = Indices::U32(indices);

            Mesh::new(
                PrimitiveTopology::TriangleList,
                vec![
                    VertexAttributeData::position(points),
                    VertexAttributeData::normal(normals),
                    VertexAttributeData::uv(uvs),
                ],
                Some(indices),
            )
        }
    }
}

fn remove_current_mesh_resources(
    render_resource_context: &dyn RenderResourceContext,
    handle: Handle<Mesh>,
) {
    //TODO: julian, use correct ids
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

#[derive(Default)]
pub struct MeshResourceProviderState {
    mesh_event_reader: EventReader<AssetEvent<Mesh>>,
}

// TODO julian: embed into resource system
pub fn get_attribute_name_id(name: &str) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    hasher.write(&name.as_bytes());
    hasher.finish() as usize //TODO: bad bad bad bad! this will likely break on 32 bit systems
}

pub fn mesh_resource_provider_system(
    mut state: Local<MeshResourceProviderState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    meshes: Res<Assets<Mesh>>,
    mesh_events: Res<Events<AssetEvent<Mesh>>>,
    mut query: Query<(&Handle<Mesh>, &mut RenderPipelines)>,
) {
    let mut changed_meshes = bevy_utils::HashSet::<Handle<Mesh>>::default();
    let render_resource_context = &**render_resource_context;
    for event in state.mesh_event_reader.iter(&mesh_events) {
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

    // update changed mesh data
    for changed_mesh_handle in changed_meshes.iter() {
        if let Some(mesh) = meshes.get(changed_mesh_handle) {
            // TODO: check for individual buffer changes
            let index_buffer = render_resource_context.create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::INDEX,
                    ..Default::default()
                },
                &mesh.get_index_buffer_bytes().unwrap(),
            );

            render_resource_context.set_asset_resource(
                *changed_mesh_handle,
                RenderResourceId::Buffer(index_buffer),
                INDEX_BUFFER_ASSET_INDEX,
            );

            for attribute in mesh.attributes.values() {
                // TODO: use a staging buffer here
                let attribute_buffer = render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                    &attribute.values.get_bytes(),
                );

                render_resource_context.set_asset_resource(
                    *changed_mesh_handle,
                    RenderResourceId::Buffer(attribute_buffer),
                    get_attribute_name_id(&attribute.name),
                );
            }
        }
    }

    // handover buffers to pipeline
    // TODO: remove this once batches are pipeline specific and deprecate assigned_meshes draw target
    for (handle, mut render_pipelines) in &mut query.iter() {
        if let Some(mesh) = meshes.get(&handle) {
            for render_pipeline in render_pipelines.pipelines.iter_mut() {
                render_pipeline.specialization.primitive_topology = mesh.primitive_topology;
            }

            if let Some(RenderResourceId::Buffer(index_buffer_resource)) =
                render_resource_context.get_asset_resource(*handle, INDEX_BUFFER_ASSET_INDEX)
            {
                // set index buffer into binding
                render_pipelines
                    .bindings
                    .set_index_buffer(index_buffer_resource);
            }

            // set vertex buffers into bindings
            for (name, _attribute) in mesh.iter_attribute() {
                let attribute_name_id = get_attribute_name_id(&name);
                if let Some(RenderResourceId::Buffer(vertex_buffer)) =
                    render_resource_context.get_asset_resource(*handle, attribute_name_id)
                {
                    render_pipelines
                        .bindings
                        .set_vertex_buffer(name.clone(), vertex_buffer);
                }
            }
        }
    }
}
