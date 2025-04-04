// Support code for clustered decals.
//
// This module provides an iterator API, which you may wish to use in your own
// shaders if you want clustered decals to provide textures other than the base
// color. The iterator API allows you to iterate over all decals affecting the
// current fragment. Use `clustered_decal_iterator_new()` and
// `clustered_decal_iterator_next()` as follows:
//
//      let view_z = get_view_z(vec4(world_position, 1.0));
//      let is_orthographic = view_is_orthographic();
//
//      let cluster_index =
//          clustered_forward::fragment_cluster_index(frag_coord, view_z, is_orthographic);
//      var clusterable_object_index_ranges =
//          clustered_forward::unpack_clusterable_object_index_ranges(cluster_index);
//
//      var iterator = clustered_decal_iterator_new(world_position, &clusterable_object_index_ranges);
//      while (clustered_decal_iterator_next(&iterator)) {
//          ... sample from the texture at iterator.texture_index at iterator.uv ...
//      }
//
// In this way, in conjunction with a custom material, you can provide your own
// texture arrays that mirror `mesh_view_bindings::clustered_decal_textures` in
// order to support decals with normal maps, etc.
//
// Note that the order in which decals are returned is currently unpredictable,
// though generally stable from frame to frame.

#define_import_path bevy_pbr::decal::clustered

#import bevy_pbr::clustered_forward
#import bevy_pbr::clustered_forward::ClusterableObjectIndexRanges
#import bevy_pbr::mesh_view_bindings
#import bevy_render::maths

// An object that allows stepping through all clustered decals that affect a
// single fragment.
struct ClusteredDecalIterator {
    // Public fields follow:
    // The index of the decal texture in the binding array.
    texture_index: i32,
    // The UV coordinates at which to sample that decal texture.
    uv: vec2<f32>,
    // A custom tag you can use for your own purposes.
    tag: u32,

    // Private fields follow:
    // The current offset of the index in the `ClusterableObjectIndexRanges` list.
    decal_index_offset: i32,
    // The end offset of the index in the `ClusterableObjectIndexRanges` list.
    end_offset: i32,
    // The world-space position of the fragment.
    world_position: vec3<f32>,
}

#ifdef CLUSTERED_DECALS_ARE_USABLE

// Creates a new iterator over the decals at the current fragment.
//
// You can retrieve `clusterable_object_index_ranges` as follows:
//
//      let view_z = get_view_z(world_position);
//      let is_orthographic = view_is_orthographic();
//
//      let cluster_index =
//          clustered_forward::fragment_cluster_index(frag_coord, view_z, is_orthographic);
//      var clusterable_object_index_ranges =
//          clustered_forward::unpack_clusterable_object_index_ranges(cluster_index);
fn clustered_decal_iterator_new(
    world_position: vec3<f32>,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>
) -> ClusteredDecalIterator {
    return ClusteredDecalIterator(
        -1,
        vec2(0.0),
        0u,
        // We subtract 1 because the first thing `decal_iterator_next` does is
        // add 1.
        i32((*clusterable_object_index_ranges).first_decal_offset) - 1,
        i32((*clusterable_object_index_ranges).last_clusterable_object_index_offset),
        world_position,
    );
}

// Populates the `iterator.texture_index` and `iterator.uv` fields for the next
// decal overlapping the current world position.
//
// Returns true if another decal was found or false if no more decals were found
// for this position.
fn clustered_decal_iterator_next(iterator: ptr<function, ClusteredDecalIterator>) -> bool {
    if ((*iterator).decal_index_offset == (*iterator).end_offset) {
        return false;
    }

    (*iterator).decal_index_offset += 1;

    while ((*iterator).decal_index_offset < (*iterator).end_offset) {
        let decal_index = i32(clustered_forward::get_clusterable_object_id(
            u32((*iterator).decal_index_offset)
        ));
        let decal_space_vector =
            (mesh_view_bindings::clustered_decals.decals[decal_index].local_from_world *
            vec4((*iterator).world_position, 1.0)).xyz;

        if (all(decal_space_vector >= vec3(-0.5)) && all(decal_space_vector <= vec3(0.5))) {
            (*iterator).texture_index =
                i32(mesh_view_bindings::clustered_decals.decals[decal_index].image_index);
            (*iterator).uv = decal_space_vector.xy * vec2(1.0, -1.0) + vec2(0.5);
            (*iterator).tag =
                mesh_view_bindings::clustered_decals.decals[decal_index].tag;
            return true;
        }

        (*iterator).decal_index_offset += 1;
    }

    return false;
}

#endif  // CLUSTERED_DECALS_ARE_USABLE

// Returns the view-space Z coordinate for the given world position.
fn get_view_z(world_position: vec3<f32>) -> f32 {
    return dot(vec4<f32>(
        mesh_view_bindings::view.view_from_world[0].z,
        mesh_view_bindings::view.view_from_world[1].z,
        mesh_view_bindings::view.view_from_world[2].z,
        mesh_view_bindings::view.view_from_world[3].z
    ), vec4(world_position, 1.0));
}

// Returns true if the current view describes an orthographic projection or
// false otherwise.
fn view_is_orthographic() -> bool {
    return mesh_view_bindings::view.clip_from_view[3].w == 1.0;
}

// Modifies the base color at the given position to account for decals.
//
// Returns the new base color with decals taken into account. If no decals
// overlap the current world position, returns the supplied base color
// unmodified.
fn apply_decal_base_color(
    world_position: vec3<f32>,
    frag_coord: vec2<f32>,
    initial_base_color: vec4<f32>,
) -> vec4<f32> {
    var base_color = initial_base_color;

#ifdef CLUSTERED_DECALS_ARE_USABLE
    // Fetch the clusterable object index ranges for this world position.

    let view_z = get_view_z(world_position);
    let is_orthographic = view_is_orthographic();

    let cluster_index =
        clustered_forward::fragment_cluster_index(frag_coord, view_z, is_orthographic);
    var clusterable_object_index_ranges =
        clustered_forward::unpack_clusterable_object_index_ranges(cluster_index);

    // Iterate over decals.

    var iterator = clustered_decal_iterator_new(world_position, &clusterable_object_index_ranges);
    while (clustered_decal_iterator_next(&iterator)) {
        // Sample the current decal.
        let decal_base_color = textureSampleLevel(
            mesh_view_bindings::clustered_decal_textures[iterator.texture_index],
            mesh_view_bindings::clustered_decal_sampler,
            iterator.uv,
            0.0
        );

        // Blend with the accumulated fragment.
        base_color = vec4(
            mix(base_color.rgb, decal_base_color.rgb, decal_base_color.a),
            base_color.a + decal_base_color.a
        );
    }
#endif  // CLUSTERED_DECALS_ARE_USABLE

    return base_color;
}

