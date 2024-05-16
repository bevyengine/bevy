use crate::{
    get_primitive_topology, vertex_attributes::convert_attribute, GltfError, GltfLoader,
    GltfLoaderSettings, RawGltf,
};
use bevy_asset::{
    io::Writer,
    saver::{AssetSaver, SavedAsset},
    AsyncWriteExt,
};
use bevy_pbr::experimental::meshlet::MeshletMesh;
use bevy_render::{
    mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
    render_asset::RenderAssetUsages,
};
use bevy_utils::{
    tracing::{debug, warn},
    HashMap,
};
use gltf::{
    json::{mesh::Primitive, Root},
    mesh::util::ReadIndices,
};

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
        let mut gltf = asset.into_json();

        for mesh in &mut gltf.meshes() {
            for primitive in &mut mesh.primitives() {
                #[cfg(feature = "meshlet_processor")]
                {
                    load_mesh(primitive, &gltf)?;

                    let meshlet_mesh = MeshletMesh::from_mesh(mesh)?;

                    // TODO: Remove old buffer/views from gltf
                    // TODO: Serialize meshlet_mesh, compress, and add buffer data to gltf
                    // TODO: Add meshlet metadata (including version number) to primitive extras
                }

                #[cfg(not(feature = "meshlet_processor"))]
                panic!(
                    "Converting GLTF files to use MeshletMeshes requires feature meshlet_processor."
                );
            }
        }

        writer.write_all(&gltf.to_vec()?).await?;

        Ok(settings.clone())
    }
}

fn load_mesh(primitive: &Primitive, gltf: &Root) -> Result<Mesh, GltfError> {
    let primitive_topology = get_primitive_topology(primitive.mode())?;

    let mut mesh = Mesh::new(primitive_topology, RenderAssetUsages::default());

    for (semantic, accessor) in primitive.attributes() {
        match convert_attribute(semantic, accessor, &gltf.buffers, &HashMap::new()) {
            Ok((attribute, values)) => mesh.insert_attribute(attribute, values),
            Err(err) => warn!("{}", err),
        }
    }

    let reader = primitive.reader(|buffer| Some(gltf.buffers[buffer.index()].as_slice()));

    if let Some(indices) = reader.read_indices() {
        mesh.insert_indices(match indices {
            ReadIndices::U8(is) => Indices::U16(is.map(|x| x as u16).collect()),
            ReadIndices::U16(is) => Indices::U16(is.collect()),
            ReadIndices::U32(is) => Indices::U32(is.collect()),
        });
    };

    if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none()
        && *matches!(mesh.primitive_topology(), PrimitiveTopology::TriangleList)
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
