// Custom material fragment shader (HLSL)
//
// Multiplies a uniform color by a texture sample to produce the final output.
// Uses register space 3 to match Bevy's MATERIAL_BIND_GROUP default.

struct PixelInput {
    float4 clip_position : SV_POSITION;
    float2 uv            : TEXCOORD0;
};

// Material uniform buffer (set MATERIAL_BIND_GROUP = space3, binding 0)
cbuffer MaterialColor : register(b0, space3) {
    float4 material_color;
};

// Material texture and sampler (set MATERIAL_BIND_GROUP = space3, bindings 1 and 2)
Texture2D    material_color_texture : register(t1, space3);
SamplerState material_color_sampler : register(s2, space3);

float4 main(PixelInput input) : SV_TARGET {
    float4 tex_color = material_color_texture.Sample(material_color_sampler, input.uv);
    return material_color * tex_color;
}
