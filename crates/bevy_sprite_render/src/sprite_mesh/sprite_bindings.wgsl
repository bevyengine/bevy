#define_import_path bevy_sprite::sprite_bindings

#import bevy_sprite::sprite_types::SpriteMaterial

#ifdef BINDLESS

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage> material_indices:
    array<bevy_sprite::sprite_types::SpriteMaterialBindings>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var<storage> material: array<SpriteMaterial>;

#else   // BINDLESS

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: SpriteMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var texture_sampler: sampler;

#endif  // BINDLESS
