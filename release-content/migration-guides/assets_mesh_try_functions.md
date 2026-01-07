---
title: "Use `Mesh::try_* mesh` functions for `Assets<Mesh>` entries when there can be `RenderAssetUsages::RENDER_WORLD`-only meshes."
pull_requests: [21732]
---

Previously, the `Assets<Mesh>` resource would not retain `RenderAssetUsages::RENDER_WORLD`-only
meshes once their data was extracted.

In 0.18, `Assets<Mesh>` retains `RenderAssetUsages::RENDER_WORLD`-only meshes, even after their data
is extracted. To handle such meshes, `Mesh` now contains `Mesh::try_*` functions which return a
`Result<..., MeshAccessError>`. These functions return an
`Err(MeshAccessError,ExtractedToRenderWorld)` when the mesh has already been extracted.

If `Assets<Mesh>` can contain `RenderAssetUsages::RENDER_WORLD`-only meshes, the following `Mesh`
functions should be changed to their `try_*` equivalent and handled appropriately:

```rust
// assets: Res<'w, Assets<Mesh>>
let mesh = assets.get(some_mesh_handle) // or assets.get_mut(some_mesh_handle)

// 0.17
mesh.insert_attribute(...)
mesh.with_inserted_attribute(...)
mesh.remove_attribute(...)
mesh.with_removed_attribute(...)
mesh.contains_attribute(...)
mesh.attribute(...)
mesh.attribute_mut(...)
mesh.attributes(...)
mesh.attributes_mut(...)
mesh.insert_indices(...)
mesh.with_inserted_indices(...)
mesh.indices(...)
mesh.indices_mut(...)
mesh.remove_indices(...)
mesh.with_removed_indices(...)
mesh.duplicate_vertices(...)
mesh.with_duplicated_vertices(...)
mesh.compute_normals(...)
mesh.compute_flat_normals(...)
mesh.compute_smooth_normals(...)
mesh.compute_area_weighted_normals(...)
mesh.compute_custom_smooth_normals(...)
mesh.with_computed_normals(...)
mesh.with_computed_flat_normals(...)
mesh.with_computed_smooth_normals(...)
mesh.with_computed_area_weighted_normals(...)
mesh.with_custom_smooth_normals(...)
mesh.transformed_by(...)
mesh.transform_by(...)
mesh.translated_by(...)
mesh.translate_by(...)
mesh.rotated_by(...)
mesh.rotate_by(...)
mesh.scaled_by(...)
mesh.scale_by(...)
mesh.normalize_joint_weights(...)
// when feature = morph enabled
mesh.has_morph_targets(...)
mesh.set_morph_targets(...)
mesh.morph_targets(...)
mesh.with_morph_targets(...)
mesh.set_morph_target_names(...)
mesh.with_morph_target_names(...)
mesh.morph_target_names(...)

// 0.18
mesh.try_insert_attribute(...)
mesh.try_with_inserted_attribute(...)
mesh.try_remove_attribute(...)
mesh.try_contains_attribute(...)
mesh.try_with_removed_attribute(...)
mesh.try_attribute_option(...) // or mesh.try_attribute(...)
mesh.try_attribute_mut_option(...) // or mesh.try_attribute_mut(...)
mesh.try_attributes(...)
mesh.try_attributes_mut(...)
mesh.try_insert_indices(...)
mesh.try_with_inserted_indices(...)
mesh.try_indices_option(...) // or mesh.try_indices(...)
mesh.try_indices_mut_option(...) // or mesh.try_indices_mut(...)
mesh.try_remove_indices(...)
mesh.try_with_removed_indices(...)
mesh.try_duplicate_vertices(...)
mesh.try_with_duplicated_vertices(...)
mesh.try_compute_normals(...)
mesh.try_compute_flat_normals(...)
mesh.try_compute_smooth_normals(...)
mesh.try_compute_area_weighted_normals(...)
mesh.try_compute_custom_smooth_normals(...)
mesh.try_with_computed_normals(...)
mesh.try_with_computed_flat_normals(...)
mesh.try_with_computed_smooth_normals(...)
mesh.try_with_computed_area_weighted_normals(...)
mesh.try_with_custom_smooth_normals(...)
mesh.try_transformed_by(...)
mesh.try_transform_by(...)
mesh.try_translated_by(...)
mesh.try_translate_by(...)
mesh.try_rotated_by(...)
mesh.try_rotate_by(...)
mesh.try_scaled_by(...)
mesh.try_scale_by(...)
mesh.try_normalize_joint_weights(...)
// when feature = morph enabled
mesh.try_has_morph_targets(...)
mesh.try_set_morph_targets(...)
mesh.try_morph_targets(...)
mesh.try_with_morph_targets(...)
mesh.try_set_morph_target_names(...)
mesh.try_with_morph_target_names(...)
mesh.try_morph_target_names(...)
// the following functions do not have a try_ equivalent, but now panic if 
// the mesh data has been extracted to `RenderWorld`.
mesh.get_vertex_size(...)
mesh.get_vertex_buffer_size(...)
mesh.get_index_buffer_bytes(...)
mesh.get_mesh_vertex_buffer_layout(...)
mesh.count_vertices(...)
mesh.create_packed_vertex_buffer_data(...)
mesh.write_packed_vertex_buffer_data(...)
```

If the calls to `Mesh` functions (under the 0.17 section) are left unchanged after upgrading to 0.18,
they will now panic if the mesh is a `RenderAssetUsages::RENDER_WORLD`-only mesh that has been
extracted to the render world.
