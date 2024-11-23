#import "shaders/skills/shared.wgsl" Vertex, VertexOutput

#if EFFECT_ID == 0
    #import "shaders/skills/sound.wgsl" frag, vert
#else if EFFECT_ID == 1
    #import "shaders/skills/orb.wgsl" frag, vert
#else if EFFECT_ID == 2
    #import "shaders/skills/slash.wgsl" frag, vert
#else if EFFECT_ID == 3
    #import "shaders/skills/railgun_trail.wgsl" frag, vert
#else if EFFECT_ID == 4
    #import "shaders/skills/magic_arrow.wgsl" frag, vert
#else if EFFECT_ID == 5
    #import "shaders/skills/hit.wgsl" frag, vert
#else if EFFECT_ID == 6
    #import "shaders/skills/lightning_ring.wgsl" frag, vert
#else if EFFECT_ID == 7
    #import "shaders/skills/lightning.wgsl" frag, vert
#endif

#import something_unused

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return frag(in);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    return vert(vertex);
}
