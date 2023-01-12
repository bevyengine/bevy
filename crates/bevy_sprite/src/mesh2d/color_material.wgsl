#import bevy_sprite::mesh2d_types
#import bevy_sprite::mesh2d_view_bindings

#ifdef PICKING
#import bevy_core_pipeline::picking
#endif

struct ColorMaterial {
    color: vec4<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
};
let COLOR_MATERIAL_FLAGS_TEXTURE_BIT: u32 = 1u;

@group(1) @binding(0)
var<uniform> material: ColorMaterial;
@group(1) @binding(1)
var texture: texture_2d<f32>;
@group(1) @binding(2)
var texture_sampler: sampler;

@group(2) @binding(0)
var<uniform> mesh: Mesh2d;

struct FragmentInput {
    #import bevy_sprite::mesh2d_vertex_output
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
#ifdef PICKING
    @location(1) picking: vec4<f32>,
#endif
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    var output_color: vec4<f32> = material.color;
#ifdef VERTEX_COLORS
    output_color = output_color * in.color;
#endif
    if ((material.flags & COLOR_MATERIAL_FLAGS_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(texture, texture_sampler, in.uv);
    }

    var out: FragmentOutput;

    out.color = output_color;

#ifdef PICKING
    out.picking = vec4(entity_index_to_vec3_f32(mesh.entity_index), picking_alpha(output_color.a));
#endif

    return out;
}
