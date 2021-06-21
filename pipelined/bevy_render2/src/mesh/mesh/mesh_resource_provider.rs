use crate::{
    mesh::{Mesh, MeshGpuData},
    render_resource::Buffer,
    renderer::RenderDevice,
};
use bevy_asset::{AssetEvent, Assets};
use bevy_ecs::prelude::*;
use bevy_utils::HashSet;
use wgpu::{util::BufferInitDescriptor, BufferUsage};

pub fn mesh_resource_provider_system(
    render_device: Res<RenderDevice>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_events: EventReader<AssetEvent<Mesh>>,
) {
    let mut changed_meshes = HashSet::default();
    for event in mesh_events.iter() {
        match event {
            AssetEvent::Created { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
                // TODO: uncomment this to support mutated meshes
                // remove_current_mesh_resources(render_resource_context, handle, &mut meshes);
            }
            AssetEvent::Removed { ref handle } => {
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_meshes.remove(handle);
            }
        }
    }

    // update changed mesh data
    for changed_mesh_handle in changed_meshes.iter() {
        if let Some(mesh) = meshes.get_mut(changed_mesh_handle) {
            // TODO: this avoids creating new meshes each frame because storing gpu data in the mesh flags it as
            // modified. this prevents hot reloading and therefore can't be used in an actual impl.
            if mesh.gpu_data.is_some() {
                continue;
            }

            let vertex_buffer_data = mesh.get_vertex_buffer_data();
            let vertex_buffer = Buffer::from(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    usage: BufferUsage::VERTEX,
                    label: None,
                    contents: &vertex_buffer_data,
                },
            ));

            let index_buffer = mesh.get_index_buffer_bytes().map(|data| {
                Buffer::from(
                    render_device.create_buffer_with_data(&BufferInitDescriptor {
                        usage: BufferUsage::INDEX,
                        contents: &data,
                        label: None,
                    }),
                )
            });

            mesh.gpu_data = Some(MeshGpuData {
                vertex_buffer,
                index_buffer,
            });
        }
    }
}
