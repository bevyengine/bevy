//! Mesh generation for voxel chunks.

use bevy_asset::RenderAssetUsages;
use bevy_color::LinearRgba;
use bevy_math::{Vec2, Vec3};
use bevy_mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};

use super::chunk::{Chunk, CHUNK_SIZE};
use super::materials::MaterialId;
use super::textures::AtlasRegion;

/// Meshing algorithm mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeshingMode {
    /// Simple cube meshing - generates individual faces for each voxel.
    #[default]
    Simple,
    /// Greedy meshing - merges adjacent faces of the same material into larger quads.
    Greedy,
}

/// Configuration for atlas-based UV mapping during mesh generation.
///
/// This struct contains only the data needed for UV computation (no asset handles),
/// so it can be safely cloned and sent to async mesh tasks.
#[derive(Clone, Debug, Default)]
pub struct AtlasUvConfig {
    /// UV regions for each material (indexed by MaterialId as u8).
    /// None means use vertex colors instead.
    pub regions: [Option<MaterialAtlasUv>; 8],
    /// Whether the atlas is configured and should be used.
    pub enabled: bool,
}

/// UV configuration for a single material.
#[derive(Clone, Copy, Debug)]
pub struct MaterialAtlasUv {
    /// The atlas region for this material.
    pub region: AtlasRegion,
    /// UV scale factor for tiling (larger = more repetition within the region).
    pub uv_scale: f32,
}

impl AtlasUvConfig {
    /// Creates a new empty atlas UV config (disabled).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the UV config for a material.
    pub fn set_material(&mut self, material: MaterialId, region: AtlasRegion, uv_scale: f32) {
        if let Some(slot) = self.regions.get_mut(material as usize) {
            *slot = Some(MaterialAtlasUv { region, uv_scale });
        }
    }

    /// Gets the UV config for a material.
    pub fn get_material(&self, material: MaterialId) -> Option<&MaterialAtlasUv> {
        self.regions.get(material as usize).and_then(|m| m.as_ref())
    }

    /// Enables the atlas.
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }
}

/// Generated mesh data for a chunk.
pub struct ChunkMesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals.
    pub normals: Vec<[f32; 3]>,
    /// Vertex UVs.
    pub uvs: Vec<[f32; 2]>,
    /// Vertex colors.
    pub colors: Vec<[f32; 4]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    /// Creates a new empty chunk mesh.
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Returns true if this mesh has no geometry.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Converts this chunk mesh to a Bevy mesh.
    pub fn to_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(self.positions),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(self.normals),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            VertexAttributeValues::Float32x2(self.uvs),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            VertexAttributeValues::Float32x4(self.colors),
        );
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

impl Default for ChunkMesh {
    fn default() -> Self {
        Self::new()
    }
}

/// Face direction for cube faces.
#[derive(Clone, Copy, Debug)]
enum Face {
    /// +X face
    PosX,
    /// -X face
    NegX,
    /// +Y face
    PosY,
    /// -Y face
    NegY,
    /// +Z face
    PosZ,
    /// -Z face
    NegZ,
}

impl Face {
    /// Returns the normal vector for this face.
    fn normal(&self) -> [f32; 3] {
        match self {
            Face::PosX => [1.0, 0.0, 0.0],
            Face::NegX => [-1.0, 0.0, 0.0],
            Face::PosY => [0.0, 1.0, 0.0],
            Face::NegY => [0.0, -1.0, 0.0],
            Face::PosZ => [0.0, 0.0, 1.0],
            Face::NegZ => [0.0, 0.0, -1.0],
        }
    }

    /// Returns the neighbor offset for this face.
    fn offset(&self) -> (i32, i32, i32) {
        match self {
            Face::PosX => (1, 0, 0),
            Face::NegX => (-1, 0, 0),
            Face::PosY => (0, 1, 0),
            Face::NegY => (0, -1, 0),
            Face::PosZ => (0, 0, 1),
            Face::NegZ => (0, 0, -1),
        }
    }
}

/// Generates a mesh for the given chunk using the specified meshing mode.
///
/// Returns `None` if the chunk is empty.
pub fn generate_chunk_mesh(chunk: &Chunk, voxel_size: f32) -> Option<Mesh> {
    generate_chunk_mesh_with_mode(chunk, voxel_size, MeshingMode::default())
}

/// Generates a mesh for the given chunk using the specified meshing mode.
///
/// Returns `None` if the chunk is empty.
pub fn generate_chunk_mesh_with_mode(
    chunk: &Chunk,
    voxel_size: f32,
    mode: MeshingMode,
) -> Option<Mesh> {
    match mode {
        MeshingMode::Simple => generate_chunk_mesh_simple(chunk, voxel_size),
        MeshingMode::Greedy => generate_chunk_mesh_greedy(chunk, voxel_size),
    }
}

/// Generates a mesh for the given chunk using simple cube meshing.
///
/// Each solid voxel gets up to 6 faces (one per exposed side).
/// Returns `None` if the chunk is empty.
fn generate_chunk_mesh_simple(chunk: &Chunk, voxel_size: f32) -> Option<Mesh> {
    if chunk.is_empty() {
        return None;
    }

    let mut mesh = ChunkMesh::new();

    for z in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let voxel = chunk.get(x, y, z);
                if !voxel.is_solid() {
                    continue;
                }

                let color = material_to_color(voxel.material);
                let base_pos = Vec3::new(
                    x as f32 * voxel_size,
                    y as f32 * voxel_size,
                    z as f32 * voxel_size,
                );

                // Check each face
                for face in [
                    Face::PosX,
                    Face::NegX,
                    Face::PosY,
                    Face::NegY,
                    Face::PosZ,
                    Face::NegZ,
                ] {
                    if should_render_face(chunk, x, y, z, face) {
                        add_face(&mut mesh, base_pos, voxel_size, face, color);
                    }
                }
            }
        }
    }

    if mesh.is_empty() {
        None
    } else {
        Some(mesh.to_mesh())
    }
}

/// Generates a mesh for the given chunk using greedy meshing.
///
/// Greedy meshing merges adjacent faces of the same material into larger quads,
/// significantly reducing polygon count. The algorithm processes each 2D slice
/// of the chunk for each face direction.
///
/// Returns `None` if the chunk is empty.
pub fn generate_chunk_mesh_greedy(chunk: &Chunk, voxel_size: f32) -> Option<Mesh> {
    generate_chunk_mesh_greedy_with_atlas(chunk, voxel_size, None)
}

/// Generates a mesh for the given chunk using greedy meshing with optional atlas UV mapping.
///
/// When `atlas_config` is provided and enabled, UVs are computed to sample from
/// the correct region of a texture atlas based on each face's material.
///
/// Returns `None` if the chunk is empty.
pub fn generate_chunk_mesh_greedy_with_atlas(
    chunk: &Chunk,
    voxel_size: f32,
    atlas_config: Option<&AtlasUvConfig>,
) -> Option<Mesh> {
    if chunk.is_empty() {
        return None;
    }

    let mut mesh = ChunkMesh::new();

    // Process each face direction
    for face in [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ] {
        greedy_mesh_face_with_atlas(chunk, &mut mesh, voxel_size, face, atlas_config);
    }

    if mesh.is_empty() {
        None
    } else {
        Some(mesh.to_mesh())
    }
}

/// Processes a single face direction using greedy meshing.
fn greedy_mesh_face(chunk: &Chunk, mesh: &mut ChunkMesh, voxel_size: f32, face: Face) {
    // Determine the axes for this face direction
    let (depth_axis, width_axis, height_axis) = match face {
        Face::PosX | Face::NegX => (0, 2, 1), // depth=X, width=Z, height=Y
        Face::PosY | Face::NegY => (1, 0, 2), // depth=Y, width=X, height=Z
        Face::PosZ | Face::NegZ => (2, 0, 1), // depth=Z, width=X, height=Y
    };

    // Create a visited mask for this face direction
    let mut visited = [[[false; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

    // Iterate through each slice perpendicular to the face normal
    for depth in 0..CHUNK_SIZE {
        // Iterate through the 2D slice
        for height in 0..CHUNK_SIZE {
            for width in 0..CHUNK_SIZE {
                // Convert 2D slice coordinates to 3D voxel coordinates
                let mut coords = [0; 3];
                coords[depth_axis] = depth;
                coords[width_axis] = width;
                coords[height_axis] = height;
                let (x, y, z) = (coords[0], coords[1], coords[2]);

                // Skip if already visited or not solid
                if visited[x][y][z] {
                    continue;
                }

                let voxel = chunk.get(x, y, z);
                if !voxel.is_solid() {
                    continue;
                }

                // Check if this face should be rendered
                if !should_render_face(chunk, x, y, z, face) {
                    continue;
                }

                let material = voxel.material;

                // Greedily expand in width
                let mut rect_width = 1;
                while width + rect_width < CHUNK_SIZE {
                    let mut test_coords = coords;
                    test_coords[width_axis] = width + rect_width;
                    let (tx, ty, tz) = (test_coords[0], test_coords[1], test_coords[2]);

                    if visited[tx][ty][tz] {
                        break;
                    }

                    let test_voxel = chunk.get(tx, ty, tz);
                    if !test_voxel.is_solid()
                        || test_voxel.material != material
                        || !should_render_face(chunk, tx, ty, tz, face)
                    {
                        break;
                    }

                    rect_width += 1;
                }

                // Greedily expand in height
                let mut rect_height = 1;
                'height_loop: while height + rect_height < CHUNK_SIZE {
                    // Check if the entire row can be merged
                    for w in 0..rect_width {
                        let mut test_coords = coords;
                        test_coords[width_axis] = width + w;
                        test_coords[height_axis] = height + rect_height;
                        let (tx, ty, tz) = (test_coords[0], test_coords[1], test_coords[2]);

                        if visited[tx][ty][tz] {
                            break 'height_loop;
                        }

                        let test_voxel = chunk.get(tx, ty, tz);
                        if !test_voxel.is_solid()
                            || test_voxel.material != material
                            || !should_render_face(chunk, tx, ty, tz, face)
                        {
                            break 'height_loop;
                        }
                    }
                    rect_height += 1;
                }

                // Mark all voxels in the rectangle as visited
                for h in 0..rect_height {
                    for w in 0..rect_width {
                        let mut mark_coords = coords;
                        mark_coords[width_axis] = width + w;
                        mark_coords[height_axis] = height + h;
                        let (mx, my, mz) = (mark_coords[0], mark_coords[1], mark_coords[2]);
                        visited[mx][my][mz] = true;
                    }
                }

                // Add the merged quad to the mesh
                let color = material_to_color(material);
                add_greedy_face(
                    mesh,
                    x,
                    y,
                    z,
                    rect_width,
                    rect_height,
                    voxel_size,
                    face,
                    width_axis,
                    height_axis,
                    color,
                );
            }
        }
    }
}

/// Processes a single face direction using greedy meshing with atlas UV support.
fn greedy_mesh_face_with_atlas(
    chunk: &Chunk,
    mesh: &mut ChunkMesh,
    voxel_size: f32,
    face: Face,
    atlas_config: Option<&AtlasUvConfig>,
) {
    // Determine the axes for this face direction
    let (depth_axis, width_axis, height_axis) = match face {
        Face::PosX | Face::NegX => (0, 2, 1), // depth=X, width=Z, height=Y
        Face::PosY | Face::NegY => (1, 0, 2), // depth=Y, width=X, height=Z
        Face::PosZ | Face::NegZ => (2, 0, 1), // depth=Z, width=X, height=Y
    };

    // Create a visited mask for this face direction
    let mut visited = [[[false; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

    // Iterate through each slice perpendicular to the face normal
    for depth in 0..CHUNK_SIZE {
        // Iterate through the 2D slice
        for height in 0..CHUNK_SIZE {
            for width in 0..CHUNK_SIZE {
                // Convert 2D slice coordinates to 3D voxel coordinates
                let mut coords = [0; 3];
                coords[depth_axis] = depth;
                coords[width_axis] = width;
                coords[height_axis] = height;
                let (x, y, z) = (coords[0], coords[1], coords[2]);

                // Skip if already visited or not solid
                if visited[x][y][z] {
                    continue;
                }

                let voxel = chunk.get(x, y, z);
                if !voxel.is_solid() {
                    continue;
                }

                // Check if this face should be rendered
                if !should_render_face(chunk, x, y, z, face) {
                    continue;
                }

                let material = voxel.material;

                // Greedily expand in width
                let mut rect_width = 1;
                while width + rect_width < CHUNK_SIZE {
                    let mut test_coords = coords;
                    test_coords[width_axis] = width + rect_width;
                    let (tx, ty, tz) = (test_coords[0], test_coords[1], test_coords[2]);

                    if visited[tx][ty][tz] {
                        break;
                    }

                    let test_voxel = chunk.get(tx, ty, tz);
                    if !test_voxel.is_solid()
                        || test_voxel.material != material
                        || !should_render_face(chunk, tx, ty, tz, face)
                    {
                        break;
                    }

                    rect_width += 1;
                }

                // Greedily expand in height
                let mut rect_height = 1;
                'height_loop: while height + rect_height < CHUNK_SIZE {
                    // Check if the entire row can be merged
                    for w in 0..rect_width {
                        let mut test_coords = coords;
                        test_coords[width_axis] = width + w;
                        test_coords[height_axis] = height + rect_height;
                        let (tx, ty, tz) = (test_coords[0], test_coords[1], test_coords[2]);

                        if visited[tx][ty][tz] {
                            break 'height_loop;
                        }

                        let test_voxel = chunk.get(tx, ty, tz);
                        if !test_voxel.is_solid()
                            || test_voxel.material != material
                            || !should_render_face(chunk, tx, ty, tz, face)
                        {
                            break 'height_loop;
                        }
                    }
                    rect_height += 1;
                }

                // Mark all voxels in the rectangle as visited
                for h in 0..rect_height {
                    for w in 0..rect_width {
                        let mut mark_coords = coords;
                        mark_coords[width_axis] = width + w;
                        mark_coords[height_axis] = height + h;
                        let (mx, my, mz) = (mark_coords[0], mark_coords[1], mark_coords[2]);
                        visited[mx][my][mz] = true;
                    }
                }

                // Add the merged quad to the mesh with atlas support
                let color = material_to_color(material);
                let atlas_uv = atlas_config
                    .filter(|c| c.enabled)
                    .and_then(|c| c.get_material(material));

                add_greedy_face_with_atlas(
                    mesh,
                    x,
                    y,
                    z,
                    rect_width,
                    rect_height,
                    voxel_size,
                    face,
                    width_axis,
                    height_axis,
                    color,
                    atlas_uv,
                );
            }
        }
    }
}

/// Adds a greedily-merged face quad to the mesh.
#[allow(clippy::too_many_arguments)]
fn add_greedy_face(
    mesh: &mut ChunkMesh,
    x: usize,
    y: usize,
    z: usize,
    width: usize,
    height: usize,
    voxel_size: f32,
    face: Face,
    width_axis: usize,
    height_axis: usize,
    color: [f32; 4],
) {
    let base_index = mesh.positions.len() as u32;
    let normal = face.normal();

    // Calculate base position
    let base_pos = Vec3::new(
        x as f32 * voxel_size,
        y as f32 * voxel_size,
        z as f32 * voxel_size,
    );

    // Get the 4 vertices for this merged face
    let vertices = get_greedy_face_vertices(
        base_pos,
        voxel_size,
        face,
        width,
        height,
        width_axis,
        height_axis,
    );

    for vertex in &vertices {
        mesh.positions.push(*vertex);
        mesh.normals.push(normal);
        mesh.colors.push(color);
    }

    // UVs for the face (scaled by rectangle size)
    let u_scale = width as f32;
    let v_scale = height as f32;
    mesh.uvs.push([0.0, 0.0]);
    mesh.uvs.push([u_scale, 0.0]);
    mesh.uvs.push([u_scale, v_scale]);
    mesh.uvs.push([0.0, v_scale]);

    // Two triangles for the quad (CCW winding)
    mesh.indices.push(base_index);
    mesh.indices.push(base_index + 1);
    mesh.indices.push(base_index + 2);
    mesh.indices.push(base_index);
    mesh.indices.push(base_index + 2);
    mesh.indices.push(base_index + 3);
}

/// Adds a greedily-merged face quad to the mesh with atlas UV support.
#[allow(clippy::too_many_arguments)]
fn add_greedy_face_with_atlas(
    mesh: &mut ChunkMesh,
    x: usize,
    y: usize,
    z: usize,
    width: usize,
    height: usize,
    voxel_size: f32,
    face: Face,
    width_axis: usize,
    height_axis: usize,
    color: [f32; 4],
    atlas_uv: Option<&MaterialAtlasUv>,
) {
    let base_index = mesh.positions.len() as u32;
    let normal = face.normal();

    // Calculate base position
    let base_pos = Vec3::new(
        x as f32 * voxel_size,
        y as f32 * voxel_size,
        z as f32 * voxel_size,
    );

    // Get the 4 vertices for this merged face
    let vertices = get_greedy_face_vertices(
        base_pos,
        voxel_size,
        face,
        width,
        height,
        width_axis,
        height_axis,
    );

    for vertex in &vertices {
        mesh.positions.push(*vertex);
        mesh.normals.push(normal);
        mesh.colors.push(color);
    }

    // Generate UVs based on atlas config or use simple tiling
    let uvs = if let Some(atlas) = atlas_uv {
        // Atlas mode: map UVs to the material's region with tiling
        let region = &atlas.region;
        let uv_scale = atlas.uv_scale;

        // Calculate tiled local UVs (these repeat within the region)
        let u_tiles = width as f32 * uv_scale;
        let v_tiles = height as f32 * uv_scale;

        // Map the 4 corners to the atlas region with tiling
        // We use fract() implicitly through the shader's texture wrap mode,
        // but we encode the atlas region bounds so the shader can sample correctly.
        // For now, we map to the region center and let tiling happen at edges.
        [
            region.map_uv_tiled(Vec2::new(0.0, 0.0)).to_array(),
            region.map_uv_tiled(Vec2::new(u_tiles, 0.0)).to_array(),
            region.map_uv_tiled(Vec2::new(u_tiles, v_tiles)).to_array(),
            region.map_uv_tiled(Vec2::new(0.0, v_tiles)).to_array(),
        ]
    } else {
        // Non-atlas mode: simple tiling UVs
        let u_scale = width as f32;
        let v_scale = height as f32;
        [
            [0.0, 0.0],
            [u_scale, 0.0],
            [u_scale, v_scale],
            [0.0, v_scale],
        ]
    };

    for uv in &uvs {
        mesh.uvs.push(*uv);
    }

    // Two triangles for the quad (CCW winding)
    mesh.indices.push(base_index);
    mesh.indices.push(base_index + 1);
    mesh.indices.push(base_index + 2);
    mesh.indices.push(base_index);
    mesh.indices.push(base_index + 2);
    mesh.indices.push(base_index + 3);
}

/// Gets the 4 vertices for a greedily-merged face in CCW winding order.
#[allow(clippy::too_many_arguments)]
fn get_greedy_face_vertices(
    base: Vec3,
    voxel_size: f32,
    face: Face,
    width: usize,
    height: usize,
    width_axis: usize,
    height_axis: usize,
) -> [[f32; 3]; 4] {
    let width_size = width as f32 * voxel_size;
    let height_size = height as f32 * voxel_size;

    // Calculate width and height offset vectors
    let mut width_offset = [0.0; 3];
    let mut height_offset = [0.0; 3];
    width_offset[width_axis] = width_size;
    height_offset[height_axis] = height_size;

    let width_vec = Vec3::from_array(width_offset);
    let height_vec = Vec3::from_array(height_offset);

    match face {
        Face::PosX => {
            let face_base = base + Vec3::new(voxel_size, 0.0, 0.0);
            [
                face_base.into(),
                (face_base + width_vec).into(),
                (face_base + width_vec + height_vec).into(),
                (face_base + height_vec).into(),
            ]
        }
        Face::NegX => {
            let face_base = base;
            [
                (face_base + width_vec).into(),
                face_base.into(),
                (face_base + height_vec).into(),
                (face_base + width_vec + height_vec).into(),
            ]
        }
        Face::PosY => {
            let face_base = base + Vec3::new(0.0, voxel_size, 0.0);
            [
                face_base.into(),
                (face_base + width_vec).into(),
                (face_base + width_vec + height_vec).into(),
                (face_base + height_vec).into(),
            ]
        }
        Face::NegY => {
            let face_base = base;
            [
                (face_base + height_vec).into(),
                (face_base + width_vec + height_vec).into(),
                (face_base + width_vec).into(),
                face_base.into(),
            ]
        }
        Face::PosZ => {
            let face_base = base + Vec3::new(0.0, 0.0, voxel_size);
            [
                face_base.into(),
                (face_base + height_vec).into(),
                (face_base + width_vec + height_vec).into(),
                (face_base + width_vec).into(),
            ]
        }
        Face::NegZ => {
            let face_base = base;
            [
                (face_base + width_vec).into(),
                (face_base + width_vec + height_vec).into(),
                (face_base + height_vec).into(),
                face_base.into(),
            ]
        }
    }
}

/// Checks if a face should be rendered (i.e., the neighbor is empty or out of bounds).
fn should_render_face(chunk: &Chunk, x: usize, y: usize, z: usize, face: Face) -> bool {
    let (dx, dy, dz) = face.offset();
    let nx = x as i32 + dx;
    let ny = y as i32 + dy;
    let nz = z as i32 + dz;

    // If neighbor is out of bounds, render the face
    if nx < 0
        || nx >= CHUNK_SIZE as i32
        || ny < 0
        || ny >= CHUNK_SIZE as i32
        || nz < 0
        || nz >= CHUNK_SIZE as i32
    {
        return true;
    }

    // If neighbor is empty, render the face
    !chunk.get(nx as usize, ny as usize, nz as usize).is_solid()
}

/// Converts a material ID to a color array.
fn material_to_color(material: MaterialId) -> [f32; 4] {
    let color: LinearRgba = material.color().into();
    [color.red, color.green, color.blue, color.alpha]
}

/// Adds a face to the mesh.
fn add_face(mesh: &mut ChunkMesh, base_pos: Vec3, size: f32, face: Face, color: [f32; 4]) {
    let base_index = mesh.positions.len() as u32;
    let normal = face.normal();

    // Get the 4 vertices for this face
    let vertices = get_face_vertices(base_pos, size, face);

    for vertex in &vertices {
        mesh.positions.push(*vertex);
        mesh.normals.push(normal);
        mesh.colors.push(color);
    }

    // UVs for the face
    mesh.uvs.push([0.0, 0.0]);
    mesh.uvs.push([1.0, 0.0]);
    mesh.uvs.push([1.0, 1.0]);
    mesh.uvs.push([0.0, 1.0]);

    // Two triangles for the quad (CCW winding)
    mesh.indices.push(base_index);
    mesh.indices.push(base_index + 1);
    mesh.indices.push(base_index + 2);
    mesh.indices.push(base_index);
    mesh.indices.push(base_index + 2);
    mesh.indices.push(base_index + 3);
}

/// Gets the 4 vertices for a face in CCW winding order.
fn get_face_vertices(base: Vec3, size: f32, face: Face) -> [[f32; 3]; 4] {
    match face {
        Face::PosX => [
            [base.x + size, base.y, base.z],
            [base.x + size, base.y, base.z + size],
            [base.x + size, base.y + size, base.z + size],
            [base.x + size, base.y + size, base.z],
        ],
        Face::NegX => [
            [base.x, base.y, base.z + size],
            [base.x, base.y, base.z],
            [base.x, base.y + size, base.z],
            [base.x, base.y + size, base.z + size],
        ],
        Face::PosY => [
            [base.x, base.y + size, base.z],
            [base.x + size, base.y + size, base.z],
            [base.x + size, base.y + size, base.z + size],
            [base.x, base.y + size, base.z + size],
        ],
        Face::NegY => [
            [base.x, base.y, base.z + size],
            [base.x + size, base.y, base.z + size],
            [base.x + size, base.y, base.z],
            [base.x, base.y, base.z],
        ],
        Face::PosZ => [
            [base.x, base.y, base.z + size],
            [base.x, base.y + size, base.z + size],
            [base.x + size, base.y + size, base.z + size],
            [base.x + size, base.y, base.z + size],
        ],
        Face::NegZ => [
            [base.x + size, base.y, base.z],
            [base.x + size, base.y + size, base.z],
            [base.x, base.y + size, base.z],
            [base.x, base.y, base.z],
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_chunk_no_mesh() {
        let chunk = Chunk::new();
        assert!(generate_chunk_mesh(&chunk, 1.0).is_none());
    }

    #[test]
    fn test_empty_chunk_no_mesh_greedy() {
        let chunk = Chunk::new();
        assert!(generate_chunk_mesh_greedy(&chunk, 1.0).is_none());
    }

    #[test]
    fn test_single_voxel_mesh() {
        let mut chunk = Chunk::new();
        chunk.set(8, 8, 8, super::super::voxel::Voxel::solid(MaterialId::Dirt));

        let mesh = generate_chunk_mesh(&chunk, 1.0);
        assert!(mesh.is_some());
    }

    #[test]
    fn test_single_voxel_mesh_greedy() {
        let mut chunk = Chunk::new();
        chunk.set(8, 8, 8, super::super::voxel::Voxel::solid(MaterialId::Dirt));

        let mesh = generate_chunk_mesh_greedy(&chunk, 1.0);
        assert!(mesh.is_some());
    }

    #[test]
    fn test_filled_chunk_mesh() {
        let chunk = Chunk::filled(MaterialId::Dirt);
        let mesh = generate_chunk_mesh(&chunk, 1.0);
        assert!(mesh.is_some());
    }

    #[test]
    fn test_filled_chunk_mesh_greedy() {
        let chunk = Chunk::filled(MaterialId::Dirt);
        let mesh = generate_chunk_mesh_greedy(&chunk, 1.0);
        assert!(mesh.is_some());
    }

    #[test]
    fn test_meshing_mode_simple() {
        let mut chunk = Chunk::new();
        chunk.set(0, 0, 0, super::super::voxel::Voxel::solid(MaterialId::Dirt));

        let mesh = generate_chunk_mesh_with_mode(&chunk, 1.0, MeshingMode::Simple);
        assert!(mesh.is_some());
    }

    #[test]
    fn test_meshing_mode_greedy() {
        let mut chunk = Chunk::new();
        chunk.set(0, 0, 0, super::super::voxel::Voxel::solid(MaterialId::Dirt));

        let mesh = generate_chunk_mesh_with_mode(&chunk, 1.0, MeshingMode::Greedy);
        assert!(mesh.is_some());
    }

    #[test]
    fn test_greedy_meshing_reduces_polygons() {
        // Create a 4x4 flat plane of voxels (should merge into 1 quad per face)
        let mut chunk = Chunk::new();
        for x in 0..4 {
            for z in 0..4 {
                chunk.set(x, 0, z, super::super::voxel::Voxel::solid(MaterialId::Dirt));
            }
        }

        // Generate both meshes
        let simple_mesh = generate_chunk_mesh_simple(&chunk, 1.0).unwrap();
        let greedy_mesh = generate_chunk_mesh_greedy(&chunk, 1.0).unwrap();

        // Get vertex counts (as a proxy for polygon count)
        let simple_vertices = simple_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let greedy_vertices = greedy_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();

        let simple_count = match simple_vertices {
            VertexAttributeValues::Float32x3(v) => v.len(),
            _ => 0,
        };

        let greedy_count = match greedy_vertices {
            VertexAttributeValues::Float32x3(v) => v.len(),
            _ => 0,
        };

        // Greedy meshing should produce significantly fewer vertices
        assert!(
            greedy_count < simple_count,
            "Greedy meshing should reduce vertex count: greedy={}, simple={}",
            greedy_count,
            simple_count
        );
    }
}
