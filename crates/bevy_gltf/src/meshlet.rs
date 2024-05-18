use crate::{
    get_primitive_topology, vertex_attributes::convert_attribute, GltfError, GltfLoader,
    GltfLoaderSettings, RawGltf,
};
use base64::{prelude::BASE64_STANDARD, Engine};
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
use gltf::{mesh::util::ReadIndices, Primitive};
use serde_json::{json, Map};
use std::{collections::VecDeque, ops::Deref};

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
        panic!("Converting GLTF files to use MeshletMeshes requires feature meshlet_processor.");

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

        let mut gltf = asset.gltf.deref().clone().into_json();

        for mesh in &mut gltf.meshes {
            for primitive in &mut mesh.primitives {
                let meshlet_mesh_bytes = meshlet_meshes.pop_front().unwrap();
                let meshlet_mesh_bytes = meshlet_mesh_bytes.into_bytes().expect("TODO");
                let meshlet_mesh_bytes = BASE64_STANDARD.encode(meshlet_mesh_bytes);

                if primitive.extensions.is_none() {
                    primitive.extensions =
                        Some(gltf::json::extensions::mesh::Primitive { others: Map::new() });
                }

                primitive.extensions.as_mut().unwrap().others.insert(
                    "BEVY_meshlet_mesh".to_owned(),
                    json!({
                        "version": MESHLET_MESH_ASSET_VERSION,
                        "bytes": meshlet_mesh_bytes,
                    }),
                );

                // TODO: Remove primitive indices, attributes, buffer views, and buffers
            }
        }

        let bytes = gltf.to_vec().expect("TODO");
        writer.write_all(&bytes).await?;

        Ok(settings.clone())
    }
}

fn load_mesh<'a>(primitive: &Primitive<'a>, buffer_data: &Vec<Vec<u8>>) -> Result<Mesh, GltfError> {
    let primitive_topology = get_primitive_topology(primitive.mode())?;

    let mut mesh = Mesh::new(primitive_topology, RenderAssetUsages::default());

    for (semantic, accessor) in primitive.attributes() {
        match convert_attribute(semantic, accessor, &buffer_data, &HashMap::new()) {
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
        mesh.compute_flat_normals();
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
