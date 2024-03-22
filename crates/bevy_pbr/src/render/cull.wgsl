#import bevy_pbr::mesh_types::Mesh
#import bevy_render::view::View
#import bevy_render::maths

struct IndirectParameters {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_index_or_vertex: u32,
    first_vertex_or_instance: u32,
    first_instance: u32,
}

struct IndirectInstanceDescriptor {
    parameters_index: u32,
    instance_index: u32,
}

struct MeshIndirect {
    aabb_center: vec3<f32>,
    aabb_half_extents: vec3<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage> meshes: array<Mesh>;
@group(0) @binding(2) var<storage, read_write> indirect_instances: array<u32>;
@group(0) @binding(3) var<storage> indirect_descriptors: array<IndirectInstanceDescriptor>;
@group(0) @binding(4) var<storage> indirect_meshes: array<MeshIndirect>;
@group(0) @binding(5) var<storage, read_write> indirect_parameters: array<IndirectParameters>;

fn transform_radius(transform: mat4x4<f32>, half_extents: vec3<f32>) -> f32 {
    return length(maths::mat4x4_to_mat3x3(transform) * half_extents);
}

fn view_frustum_intersects_sphere(sphere_center: vec3<f32>, sphere_radius: f32) -> bool {
    let center = vec4<f32>(sphere_center, 1.0);
    for (var i = 0; i < 5; i += 1) {
        if (dot(view.frustum[i], center) + sphere_radius <= 0.0) {
            return false;
        }
    }
    return true;
}

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let mesh_instance_index = global_invocation_id.x;
    if (mesh_instance_index >= arrayLength(&indirect_descriptors)) {
        return;
    }

    let indirect_descriptor = indirect_descriptors[mesh_instance_index];
    let instance_index = indirect_descriptor.instance_index;

    let indirect_data_index = meshes[instance_index].indirect_data_index;
    let mesh_indirect = indirect_meshes[indirect_data_index];

    let model = maths::affine3_to_square(meshes[instance_index].model);

    let sphere_center = (model * vec4(mesh_indirect.aabb_center, 1.0)).xyz;
    let sphere_radius = transform_radius(model, mesh_indirect.aabb_half_extents);
    if (!view_frustum_intersects_sphere(sphere_center, sphere_radius)) {
        return;
    }

    let parameters_index = indirect_descriptor.parameters_index;
    let instance_handle = atomicAdd(&indirect_parameters[parameters_index].instance_count, 1u) +
        indirect_parameters[parameters_index].first_instance;

    indirect_instances[instance_handle] = indirect_descriptor.instance_index;
}
