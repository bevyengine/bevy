#import bevy_sprite::{    
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_POSITIONS
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let position = vec4<f32>(vertex.position * vec3<f32>(material.vertex_scale, 1.0) + vec3<f32>(material.vertex_offset, 0.0), 1.0);

    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        position
    );
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
#endif

#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh2d_normal_local_to_world(vertex.normal, vertex.instance_index);
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(
        world_from_local,
        vertex.tangent
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
    return out;
}

struct SpriteMaterial {
    color: vec4<f32>,
    flags: u32,
    alpha_cutoff: f32, 
    vertex_scale: vec2<f32>,
    vertex_offset: vec2<f32>,
    uv_transform: mat3x3<f32>,
    
    tile_stretch_value: vec2<f32>,

    scale: vec2<f32>,
    min_inset: vec2<f32>,
    max_inset: vec2<f32>,
    side_stretch_value: f32,
    center_stretch_value: f32,
    corner_scale: f32,
};

const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS: u32 = 3221225472u; // (0b11u32 << 30)
const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32        = 0u;          // (0u32 << 30)
const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32          = 1073741824u; // (1u32 << 30)
const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32         = 2147483648u; // (2u32 << 30)

const SPRITE_MATERIAL_FLAGS_FLIP_X: u32                   = 1u;
const SPRITE_MATERIAL_FLAGS_FLIP_Y: u32                   = 2u;
const SPRITE_MATERIAL_FLAGS_TILE_X: u32                   = 4u;
const SPRITE_MATERIAL_FLAGS_TILE_Y: u32                   = 8u;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: SpriteMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var texture_sampler: sampler;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {

    var uv = mesh.uv; 

    if (material.flags & SPRITE_MATERIAL_FLAGS_FLIP_X) != 0u {
        uv.x = 1.0 - uv.x;
    }
    if (material.flags & SPRITE_MATERIAL_FLAGS_FLIP_Y) != 0u {
        uv.y = 1.0 - uv.y;
    }

    if (material.flags & SPRITE_MATERIAL_FLAGS_TILE_X) != 0u {
        uv.x = (uv.x - material.tile_stretch_value.x * floor(uv.x / material.tile_stretch_value.x)) / material.tile_stretch_value.x;
    }
    if (material.flags & SPRITE_MATERIAL_FLAGS_TILE_Y) != 0u {
        uv.y = (uv.y - material.tile_stretch_value.y * floor(uv.y / material.tile_stretch_value.y)) / material.tile_stretch_value.y;
    }

    // using this as a temp check for slicing
    if material.scale.x != 0.0 {
        uv = apply_slicing(uv);
    }

    uv = (material.uv_transform * vec3(uv, 1.0)).xy;

    let sprite_color = textureSample(texture, texture_sampler, uv);
    var output_color = alpha_discard(sprite_color * material.color); 

#ifdef TONEMAP_IN_SHADER
    output_color = tonemapping::tone_mapping(output_color, view.color_grading);
#endif
    
    return output_color;
}

fn apply_slicing(uv: vec2<f32>) -> vec2<f32> {
    let left = uv.x < material.min_inset.x * material.corner_scale;
    let right = uv.x > 1.0 - material.max_inset.x * material.corner_scale;
    let top = uv.y < material.max_inset.y * material.corner_scale; 
    let bottom = uv.y > 1.0 - material.max_inset.y * material.corner_scale;

    let corner_scale_value = material.scale / material.corner_scale;

    // top-left corner
    if top && left {
        return uv * corner_scale_value; 
    } 

    // top-right corner
    if top && right { 
        return vec2<f32>(
            1.0 - (1.0 - uv.x) * corner_scale_value.x,  
            uv.y * corner_scale_value.y
        );
    }

    // bottom-left corner
    if bottom && left {
        return vec2<f32>(
            uv.x * corner_scale_value.x, 
            1.0 - (1.0 - uv.y) * corner_scale_value.y
        );
    }

    // bottom-right corner
    if bottom && right {
        return vec2<f32>(1.0) - (vec2<f32>(1.0) - uv) * corner_scale_value;
    }

    // top edge
    if top {
        return vec2<f32>(
            (uv.x - material.min_inset.x * material.corner_scale) / (1.0 - material.max_inset.x * material.corner_scale - material.min_inset.x * material.corner_scale) * ((1.0 - material.max_inset.x * material.scale.x - material.min_inset.x * material.scale.x)) + material.min_inset.x * material.scale.x, // + material.min_inset.x,
            uv.y * corner_scale_value.y
        );
    }

    return vec2<f32>(0.5);
}

fn alpha_discard(output_color: vec4<f32>) -> vec4<f32> {
    var color = output_color;
    let alpha_mode = material.flags & SPRITE_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    
    if alpha_mode == SPRITE_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        color.a = 1.0;
    }
    
#ifdef MAY_DISCARD
    else if alpha_mode == SPRITE_MATERIAL_FLAGS_ALPHA_MODE_MASK {
    if color.a >= material.alpha_cutoff {
            // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
            color.a = 1.0;
        } else {
            // NOTE: output_color.a < in.material.alpha_cutoff should not be rendered
            discard;
        }
    }
#endif // MAY_DISCARD

    return color;
}
