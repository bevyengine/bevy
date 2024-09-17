#import bevy_render::view::View;
#import bevy_render::globals::Globals;

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<uniform> globals: Globals;

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,

    // Defines the dividing line that are used to split the texture atlas rect into corner, side and center slices
    // The distances are normalized and from the top left corner of the texture atlas rect
    // x = distance of the left vertical dividing line
    // y = distance of the top horizontal dividing line
    // z = distance of the right vertical dividing line
    // w = distance of the bottom horizontal dividing line
    @location(2) @interpolate(flat) texture_slices: vec4<f32>,

    // Defines the dividing line that are used to split the render target into into corner, side and center slices
    // The distances are normalized and from the top left corner of the render target
    // x = distance of left vertical dividing line
    // y = distance of top horizontal dividing line
    // z = distance of right vertical dividing line
    // w = distance of bottom horizontal dividing line
    @location(3) @interpolate(flat) target_slices: vec4<f32>,
    
    // The number of times the side or center texture slices should be repeated when mapping them to the border slices
    // x = number of times to repeat along the horizontal axis for the side textures
    // y = number of times to repeat along the vertical axis for the side textures
    // z = number of times to repeat along the horizontal axis for the center texture
    // w = number of times to repeat along the vertical axis for the center texture
    @location(4) @interpolate(flat) repeat: vec4<f32>,

    // normalized texture atlas rect coordinates
    // x, y = top, left corner of the atlas rect
    // z, w = bottom, right corner of the atlas rect
    @location(5) @interpolate(flat) atlas_rect: vec4<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) texture_slices: vec4<f32>,
    @location(4) target_slices: vec4<f32>,
    @location(5) repeat: vec4<f32>,
    @location(6) atlas_rect: vec4<f32>,
) -> UiVertexOutput {
    var out: UiVertexOutput;
    out.uv = vertex_uv;
    out.color = vertex_color;
    out.position = view.clip_from_world * vec4<f32>(vertex_position, 1.0);
    out.texture_slices = texture_slices;
    out.target_slices = target_slices;
    out.repeat = repeat;
    out.atlas_rect = atlas_rect;
    return out;
}

/// maps a point along the axis of the render target to slice coordinates
fn map_axis_with_repeat(
    // normalized distance along the axis
    p: f32,
    // target min dividing point
    il: f32,
    // target max dividing point
    ih: f32,
    // slice min dividing point
    tl: f32,
    // slice max dividing point
    th: f32,
    // number of times to repeat the slice for sides and the center
    r: f32,
) -> f32 {
    if p < il {
        // inside one of the two left (horizontal axis) or top (vertical axis) corners
        return (p / il) * tl;
    } else if ih < p {
        // inside one of the two (horizontal axis) or top (vertical axis) corners
        return th + ((p - ih) / (1 - ih)) * (1 - th);
    } else {
        // not inside a corner, repeat the texture
        return tl + fract((r * (p - il)) / (ih - il)) * (th - tl);
    }
}

fn map_uvs_to_slice(
    uv: vec2<f32>,
    target_slices: vec4<f32>,
    texture_slices: vec4<f32>,
    repeat: vec4<f32>,
) -> vec2<f32> {
    var r: vec2<f32>;
    if target_slices.x <= uv.x && uv.x <= target_slices.z && target_slices.y <= uv.y && uv.y <= target_slices.w {
        // use the center repeat values if the uv coords are inside the center slice of the target
        r = repeat.zw;
    } else {
        // use the side repeat values if the uv coords are outside the center slice
        r = repeat.xy;
    }

    // map horizontal axis
    let x = map_axis_with_repeat(uv.x, target_slices.x, target_slices.z, texture_slices.x, texture_slices.z, r.x);

    // map vertical axis
    let y = map_axis_with_repeat(uv.y, target_slices.y, target_slices.w, texture_slices.y, texture_slices.w, r.y);

    return vec2(x, y);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // map the target uvs to slice coords
    let uv = map_uvs_to_slice(in.uv, in.target_slices, in.texture_slices, in.repeat);

    // map the slice coords to texture coords
    let atlas_uv = in.atlas_rect.xy + uv * (in.atlas_rect.zw - in.atlas_rect.xy);

    return in.color * textureSample(sprite_texture, sprite_sampler, atlas_uv);
}
