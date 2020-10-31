use crate::{
    pipeline::{PrimitiveTopology, RenderPipelines, VertexFormat},
    renderer::{BufferInfo, BufferUsage, RenderResourceContext, RenderResourceId},
};
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::AsBytes;
use bevy_ecs::{Local, Query, Res, ResMut};
use bevy_math::*;
use bevy_type_registry::TypeUuid;
use std::borrow::Cow;

use crate::pipeline::{InputStepMode, VertexAttributeDescriptor, VertexBufferDescriptor};
use bevy_utils::HashMap;

pub const INDEX_BUFFER_ASSET_INDEX: u64 = 0;
pub const VERTEX_ATTRIBUTE_BUFFER_ID: u64 = 10;
pub const VERTEX_FALLBACK_BUFFER_ID: u64 = 20;
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

impl From<Vec<f32>> for VertexAttributeValues {
    fn from(vec: Vec<f32>) -> Self {
        VertexAttributeValues::Float(vec)
    }
}

impl From<Vec<[f32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 2]>) -> Self {
        VertexAttributeValues::Float2(vec)
    }
}

impl From<Vec<[f32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 3]>) -> Self {
        VertexAttributeValues::Float3(vec)
    }
}

impl From<Vec<[f32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 4]>) -> Self {
        VertexAttributeValues::Float4(vec)
    }
}

#[derive(Debug)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}
// TODO: allow values to be unloaded after been submitting to the GPU to conserve memory
pub type VertexAttributesHashMap = HashMap<Cow<'static, str>, VertexAttributeValues>;

#[derive(Debug, TypeUuid)]
#[uuid = "8ecbac0f-f545-4473-ad43-e1f4243af51e"]
pub struct Mesh {
    pub primitive_topology: PrimitiveTopology,
    /// `bevy_utils::HashMap` with all defined vertex attributes (Positions, Normals, ...) for this mesh. Attribute name maps to attribute values.
    pub attributes: VertexAttributesHashMap,
    pub indices: Option<Indices>,
    /// The layout of the attributes in the GPU buffer without `shader_location`. `None` will indicate that no data has been uploaded to the GPU yet.
    pub attribute_buffer_descriptor_reference: Option<VertexBufferDescriptor>,
}

impl Mesh {
    pub const ATTRIBUTE_NORMAL: &'static str = "Vertex_Normal";
    pub const ATTRIBUTE_POSITION: &'static str = "Vertex_Position";
    pub const ATTRIBUTE_UV_0: &'static str = "Vertex_Uv";

    pub fn new(primitive_topology: PrimitiveTopology) -> Self {
        Mesh {
            primitive_topology,
            attributes: Default::default(),
            indices: None,
            attribute_buffer_descriptor_reference: Default::default(),
        }
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
    use super::{Indices, Mesh};
    use crate::pipeline::PrimitiveTopology;
    use bevy_math::*;
    use hexasphere::Hexasphere;
    use std::borrow::Cow;

    /// A cube.
    #[derive(Debug)]
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

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_POSITION), positions.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_NORMAL), normals.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_UV_0), uvs.into());
            mesh.indices = Some(indices);
            mesh
        }
    }

    /// A rectangle on the XY plane.
    #[derive(Debug)]
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

            let mut positions = Vec::<[f32; 3]>::new();
            let mut normals = Vec::<[f32; 3]>::new();
            let mut uvs = Vec::<[f32; 2]>::new();
            for (position, normal, uv) in vertices.iter() {
                positions.push(*position);
                normals.push(*normal);
                uvs.push(*uv);
            }

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.indices = Some(indices);
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_POSITION), positions.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_NORMAL), normals.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_UV_0), uvs.into());
            mesh
        }
    }

    /// A square on the XZ plane.
    #[derive(Debug)]
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

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.indices = Some(indices);
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_POSITION), positions.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_NORMAL), normals.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_UV_0), uvs.into());
            mesh
        }
    }

    /// A sphere made from a subdivided Icosahedron.
    #[derive(Debug)]
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

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.indices = Some(indices);
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_POSITION), points.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_NORMAL), normals.into());
            mesh.attributes
                .insert(Cow::Borrowed(Mesh::ATTRIBUTE_UV_0), uvs.into());
            mesh
        }
    }
}

fn remove_resource_save(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Mesh>,
    index: u64,
) {
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(&handle, index)
    {
        render_resource_context.remove_buffer(buffer);
        render_resource_context.remove_asset_resource(handle, index);
    }
}
fn remove_current_mesh_resources(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Mesh>,
) {
    remove_resource_save(render_resource_context, handle, VERTEX_ATTRIBUTE_BUFFER_ID);
    remove_resource_save(render_resource_context, handle, VERTEX_FALLBACK_BUFFER_ID);
    remove_resource_save(render_resource_context, handle, INDEX_BUFFER_ASSET_INDEX);
}

#[derive(Default)]
pub struct MeshResourceProviderState {
    mesh_event_reader: EventReader<AssetEvent<Mesh>>,
}

pub fn mesh_resource_provider_system(
    mut state: Local<MeshResourceProviderState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_events: Res<Events<AssetEvent<Mesh>>>,
    mut query: Query<(&Handle<Mesh>, &mut RenderPipelines)>,
) {
    let mut changed_meshes = bevy_utils::HashSet::<Handle<Mesh>>::default();
    let render_resource_context = &**render_resource_context;
    for event in state.mesh_event_reader.iter(&mesh_events) {
        match event {
            AssetEvent::Created { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
                remove_current_mesh_resources(render_resource_context, handle);
            }
            AssetEvent::Removed { ref handle } => {
                remove_current_mesh_resources(render_resource_context, handle);
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_meshes.remove(handle);
            }
        }
    }

    // update changed mesh data
    for changed_mesh_handle in changed_meshes.iter() {
        if let Some(mesh) = meshes.get_mut(changed_mesh_handle) {
            // TODO: check for individual buffer changes in non-interleaved mode
            let index_buffer = render_resource_context.create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::INDEX,
                    ..Default::default()
                },
                &mesh.get_index_buffer_bytes().unwrap(),
            );

            render_resource_context.set_asset_resource(
                changed_mesh_handle,
                RenderResourceId::Buffer(index_buffer),
                INDEX_BUFFER_ASSET_INDEX,
            );

            // Vertex buffer
            let vertex_count = attributes_count_vertices(&mesh.attributes).unwrap();
            let interleaved_buffer =
                attributes_to_vertex_buffer_data(&mesh.attributes, vertex_count);

            mesh.attribute_buffer_descriptor_reference = Some(interleaved_buffer.1);
            render_resource_context.set_asset_resource(
                changed_mesh_handle,
                RenderResourceId::Buffer(render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                    &interleaved_buffer.0,
                )),
                VERTEX_ATTRIBUTE_BUFFER_ID,
            );

            // Fallback buffer
            // TODO: can be done with a 1 byte buffer + zero stride?
            render_resource_context.set_asset_resource(
                changed_mesh_handle,
                RenderResourceId::Buffer(render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::VERTEX,
                        ..Default::default()
                    },
                    &vec![0; (vertex_count * VertexFormat::Float4.get_size() as u32) as usize],
                )),
                VERTEX_FALLBACK_BUFFER_ID,
            );
        }
    }

    // handover buffers to pipeline
    // TODO: remove this once batches are pipeline specific and deprecate assigned_meshes draw target
    for (handle, mut render_pipelines) in query.iter_mut() {
        if let Some(mesh) = meshes.get(handle) {
            for render_pipeline in render_pipelines.pipelines.iter_mut() {
                render_pipeline.specialization.primitive_topology = mesh.primitive_topology;
            }

            if let Some(RenderResourceId::Buffer(index_buffer_resource)) =
                render_resource_context.get_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX)
            {
                // set index buffer into binding
                render_pipelines
                    .bindings
                    .set_index_buffer(index_buffer_resource);
            }

            if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_resource)) =
                render_resource_context.get_asset_resource(handle, VERTEX_ATTRIBUTE_BUFFER_ID)
            {
                // set index buffer into binding
                render_pipelines.bindings.vertex_attribute_buffer =
                    Some(vertex_attribute_buffer_resource);
            }
            if let Some(RenderResourceId::Buffer(vertex_attribute_fallback_resource)) =
                render_resource_context.get_asset_resource(handle, VERTEX_FALLBACK_BUFFER_ID)
            {
                // set index buffer into binding
                render_pipelines.bindings.vertex_fallback_buffer =
                    Some(vertex_attribute_fallback_resource);
            }
        }
    }
}

pub fn attributes_count_vertices(attributes: &VertexAttributesHashMap) -> Option<u32> {
    let mut vertex_count: Option<u32> = None;
    for (attribute_name, attribute_data) in attributes {
        let attribute_len = attribute_data.len();
        if let Some(previous_vertex_count) = vertex_count {
            assert_eq!(previous_vertex_count, attribute_len as u32,
                       "Attribute {} has a different vertex count ({}) than other attributes ({}) in this mesh.", attribute_name, attribute_len, previous_vertex_count);
        }
        vertex_count = Some(attribute_len as u32);
    }
    vertex_count
}
pub fn attributes_to_vertex_buffer_data(
    attributes: &VertexAttributesHashMap,
    vertex_count: u32,
) -> (Vec<u8>, VertexBufferDescriptor) {
    // get existing attribute data as bytes and generate attribute descriptor
    let mut attributes_gpu_ready = Vec::<(VertexAttributeDescriptor, &[u8])>::default();
    let mut accumulated_offset = 0;
    let mut attributes_sorted: Vec<_> = attributes.iter().collect();
    attributes_sorted.sort_by(|a, b| a.0.cmp(b.0));
    for attribute_data in attributes_sorted {
        // TODO: allow for custom converter here
        let vertex_format = VertexFormat::from(attribute_data.1);
        attributes_gpu_ready.push((
            // this serves as a reference and is not supposed to be used directly.
            VertexAttributeDescriptor {
                name: attribute_data.0.clone(),
                offset: accumulated_offset,
                format: vertex_format,
                shader_location: 0,
            },
            attribute_data.1.get_bytes(),
        ));
        accumulated_offset += vertex_format.get_size();
    }
    let mut attributes_interleaved_buffer = Vec::<u8>::default();

    // bundle into interleaved buffers
    for vertex_index in 0..vertex_count {
        let vertex_index = vertex_index as usize;
        for (attribute_descriptor, attributes_bytes) in &attributes_gpu_ready {
            let stride = attribute_descriptor.format.get_size() as usize;
            // insert one element
            attributes_interleaved_buffer
                .extend(&attributes_bytes[vertex_index * stride..vertex_index * stride + stride]);
        }
    }

    let vertex_buffer_descriptor_reference = VertexBufferDescriptor {
        name: Default::default(),
        stride: accumulated_offset,
        step_mode: InputStepMode::Vertex,
        attributes: attributes_gpu_ready.iter().map(|x| x.0.clone()).collect(),
    };

    (
        attributes_interleaved_buffer,
        vertex_buffer_descriptor_reference,
    )
}
