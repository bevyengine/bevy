
#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0) var base_texture: texture_2d<f32>;
@group(2) @binding(1) var base_sampler: sampler;
@group(2) @binding(2) var displacement_map_texture: texture_2d<f32>;
@group(2) @binding(3) var displacement_map_sampler: sampler;
@group(2) @binding(4) var<uniform> time: f32;
@group(2) @binding(5) var<uniform> time_sensitivity: f32;
@group(2) @binding(6) var<uniform> displacement_intensity: f32;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sampling the displacement map, after translating it diagonally
    let timeshift = time * time_sensitivity;
    let timeshifted_uv = fract(in.uv + vec2<f32>(timeshift, timeshift));
    let displacement = textureSample(displacement_map_texture, displacement_map_sampler, timeshifted_uv).r;

    // Adjusting displacement so that 0.5 (perfectly gray) means no displacement.
    let adjusted_displacement = (displacement - 0.5) * displacement_intensity;

    // Calculating new UVs using displacement
    let displaced_uv = in.uv + vec2<f32>(adjusted_displacement, adjusted_displacement);

    // Wrapping UV coordinates to handle overflow
    let wrapped_uv = fract(displaced_uv);

    // Sampling the base texture with corrected UVs
    return textureSample(base_texture, base_sampler, wrapped_uv);
}

