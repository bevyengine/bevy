#define_import_path bevy_sprite::sprite_bindings

#import bevy_sprite::sprite_types::SpriteMaterial

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: SpriteMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var texture_sampler: sampler;
