use crate::{
    get_primitive_topology, vertex_attributes::convert_attribute, GltfError, GltfLoader,
    GltfLoaderSettings, RawGltf,
};
use bevy_asset::{
    io::Writer,
    saver::{AssetSaver, SavedAsset},
    AsyncWriteExt,
};
use bevy_pbr::experimental::meshlet::{MeshletMesh, MESHLET_MESH_ASSET_VERSION};
use bevy_render::{
    mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
    render_asset::RenderAssetUsages,
};
use bevy_utils::{
    tracing::{debug, warn},
    HashMap,
};
use gltf::{
    json::{Buffer, Index},
    mesh::util::ReadIndices,
    Primitive,
};
use serde_json::json;
use std::{collections::VecDeque, iter, ops::Deref};

/// An [`AssetSaver`] that converts all mesh primitives in a glTF file into [`MeshletMesh`]s.
///
/// Only certain types of meshes and materials are supported. See [`MeshletMesh`] and [`MeshletMesh::from_mesh`]
/// for more details.
///
/// Using this asset saver requires enabling the `meshlet_processor` cargo feature in addition to `asset_processor`.
///
/// Use only glTF Binary (.glb) or glTF Embedded (.gltf without additional .bin) files.
/// Using glTF Separate files (.gltf with additional .bin) will lead to unnecessary data in the final processed asset.
///
/// Example asset meta file:
/// ```
/// (
///     meta_format_version: "1.0",
///     asset: Process(
///         processor: "MeshletMeshProcessor",
///         settings: (
///             loader_settings: (),
///             saver_settings: (),
///         ),
///     ),
/// )
/// ```
pub struct MeshletMeshGltfSaver;

impl AssetSaver for MeshletMeshGltfSaver {
    type Asset = RawGltf;
    type Settings = GltfLoaderSettings;
    type OutputLoader = GltfLoader;
    type Error = GltfError;

    async fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, RawGltf>,
        settings: &'a GltfLoaderSettings,
    ) -> Result<GltfLoaderSettings, GltfError> {
        #[cfg(not(feature = "meshlet_processor"))]
        panic!(
            "Converting GLTF files to use MeshletMeshes requires cargo feature meshlet_processor."
        );

        // Convert each primitive to a meshlet mesh
        let mut meshlet_meshes: VecDeque<MeshletMesh> = VecDeque::new();
        for mesh in asset.gltf.meshes() {
            for primitive in mesh.primitives() {
                let mesh = load_mesh(&primitive, &asset.buffer_data)?;

                #[cfg(feature = "meshlet_processor")]
                {
                    let meshlet_mesh = MeshletMesh::from_mesh(&mesh).expect("TODO");
                    meshlet_meshes.push_back(meshlet_mesh);
                }
            }
        }

        // Create a mutable copy of the gltf asset
        let mut gltf = asset.gltf.deref().clone().into_json();

        // Clone the GLB BIN buffer, if it exists, or make a new buffer
        let mut glb_buffer = match gltf.buffers.first() {
            Some(buffer) if buffer.uri.is_none() => asset.buffer_data[0].clone(),
            _ => Vec::new(),
        };

        // If there was not an existing GLB BIN buffer, but there were other buffers,
        // increment each buffer view's buffer index to account for the GLB BIN buffer
        // that we're going to add at index 0
        if let Some(Buffer { uri: Some(_), .. }) = gltf.buffers.first() {
            for buffer_view in &mut gltf.buffer_views {
                buffer_view.buffer = Index::new(buffer_view.buffer.value() as u32 + 1);
            }
        }

        // For each primitive, append the serialized meshlet mesh to the GLB BIN buffer,
        // and add a custom extension pointing to the newly added slice of the buffer
        for mesh in &mut gltf.meshes {
            for primitive in &mut mesh.primitives {
                let meshlet_mesh = meshlet_meshes.pop_front().unwrap();
                let mut meshlet_mesh_bytes = meshlet_mesh.into_bytes().expect("TODO");

                let extension = json!({
                    "version": MESHLET_MESH_ASSET_VERSION,
                    "byteOffset": glb_buffer.len(),
                    "byteLength": meshlet_mesh_bytes.len(),
                });

                primitive
                    .extensions
                    .get_or_insert(Default::default())
                    .others
                    .insert("BEVY_meshlet_mesh".to_owned(), extension);

                glb_buffer.append(&mut meshlet_mesh_bytes);

                // TODO: Remove primitive indices, attributes, buffer views, and buffers
            }
        }

        // Pad GLB BIN buffer if needed
        glb_buffer.extend(iter::repeat(0x00u8).take(glb_buffer.len() % 4));

        match gltf.buffers.get_mut(0) {
            // If there was an existing GLB BIN buffer, update it's size to account
            // for the newly added meshlet mesh data
            Some(buffer) if buffer.uri.is_none() => {
                buffer.byte_length = glb_buffer.len().into();
            }
            // Else insert a new GLB BIN buffer
            _ => {
                let buffer = Buffer {
                    byte_length: glb_buffer.len().into(),
                    name: None,
                    uri: None,
                    extensions: None,
                    extras: None,
                };
                gltf.buffers.insert(0, buffer);
            }
        }

        // Pad JSON buffer if needed
        let mut gltf_bytes = gltf.to_vec().expect("TODO");
        gltf_bytes.extend(iter::repeat(0x20u8).take(gltf_bytes.len() % 4));

        // Calculate total GLB file size (headers, including chunk headers, plus JSON and BIN chunk)
        let json_len = gltf_bytes.len() as u32;
        let bin_len = glb_buffer.len() as u32;
        let file_size = 28 + json_len + bin_len;

        // Write file header
        writer.write_all(&0x46546C67u32.to_le_bytes()).await?;
        writer.write_all(&2u32.to_le_bytes()).await?;
        writer.write_all(&file_size.to_le_bytes()).await?;

        // Write JSON chunk
        writer.write_all(&json_len.to_le_bytes()).await?;
        writer.write_all(&0x4E4F534Au32.to_le_bytes()).await?;
        writer.write_all(&gltf_bytes).await?;

        // Write BIN chunk
        writer.write_all(&bin_len.to_le_bytes()).await?;
        writer.write_all(&0x004E4942u32.to_le_bytes()).await?;
        writer.write_all(&glb_buffer).await?;

        Ok(settings.clone())
    }
}

fn load_mesh(primitive: &Primitive<'_>, buffer_data: &Vec<Vec<u8>>) -> Result<Mesh, GltfError> {
    let primitive_topology = get_primitive_topology(primitive.mode())?;

    let mut mesh = Mesh::new(primitive_topology, RenderAssetUsages::default());

    for (semantic, accessor) in primitive.attributes() {
        match convert_attribute(semantic, accessor, buffer_data, &HashMap::new()) {
            Ok((attribute, values)) => mesh.insert_attribute(attribute, values),
            Err(err) => warn!("{}", err),
        }
    }

    let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()].as_slice()));

    if let Some(indices) = reader.read_indices() {
        mesh.insert_indices(match indices {
            ReadIndices::U8(is) => Indices::U16(is.map(|x| x as u16).collect()),
            ReadIndices::U16(is) => Indices::U16(is.collect()),
            ReadIndices::U32(is) => Indices::U32(is.collect()),
        });
    };

    if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none()
        && matches!(mesh.primitive_topology(), PrimitiveTopology::TriangleList)
    {
        debug!("Automatically calculating missing vertex normals for geometry.");
        let vertex_count_before = mesh.count_vertices();
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();
        let vertex_count_after = mesh.count_vertices();
        if vertex_count_before != vertex_count_after {
            debug!("Missing vertex normals in indexed geometry, computing them as flat. Vertex count increased from {} to {}", vertex_count_before, vertex_count_after);
        } else {
            debug!("Missing vertex normals in indexed geometry, computing them as flat.");
        }
    }

    if let Some(vertex_attribute) = reader
        .read_tangents()
        .map(|v| VertexAttributeValues::Float32x4(v.collect()))
    {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vertex_attribute);
    } else {
        debug!("Missing vertex tangents, computing them using the mikktspace algorithm. Consider using a tool such as Blender to pre-compute the tangents.");
        mesh.generate_tangents()?;
    }

    Ok(mesh)
}
