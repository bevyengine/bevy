#define_import_path bevy_pbr::meshlet_visibility_buffer_resolve

/// Dummy shader to prevent naga_oil from complaining about missing imports when the MeshletPlugin is not loaded,
/// as naga_oil tries to resolve imports even if they're behind an #ifdef.
