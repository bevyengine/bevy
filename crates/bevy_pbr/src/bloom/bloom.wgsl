struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
	[[location(0)]] uv: vec2<f32>;
};

var<private> vertices: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
	vec2<f32>(-1.0, -1.0),
	vec2<f32>(3.0, -1.0),
	vec2<f32>(-1.0, 3.0),
);

// full screen triangle vertex shader
[[stage(vertex)]]
fn vertex([[builtin(vertex_index)]] idx: u32) -> VertexOutput {
	var out: VertexOutput;

	out.position = vec4<f32>(vertices[idx], 0.0, 1.0);
	out.uv = vertices[idx] * vec2<f32>(0.5, -0.5);
	out.uv = out.uv + 0.5;

	return out;
}

struct Uniforms {
	threshold: f32;
	knee: f32;
	scale: f32;
};

[[group(0), binding(0)]]
var org: texture_2d<f32>;

[[group(0), binding(1)]]
var org_sampler: sampler;

[[group(0), binding(2)]]
var<uniform> uniforms: Uniforms;

[[group(0), binding(3)]]
var up: texture_2d<f32>;

fn quadratic_threshold(color: vec4<f32>, threshold: f32, curve: vec3<f32>) -> vec4<f32> {
	let br = max(max(color.r, color.g), color.b);

	var rq: f32 = clamp(br - curve.x, 0.0, curve.y);
	rq = curve.z * rq * rq;

	return color * max(rq, br - threshold) / max(br, 0.0001);
}

// samples org around the supplied uv using a filter
//
// o   o   o
//   o   o
// o   o   o
//   o   o
// o   o   o
//
// this is used because it has a number of advantages that
// outway the cost of 13 samples that basically boil down
// to it looking better
//
// these advantages are outlined in a youtube video by the Cherno:
//   https://www.youtube.com/watch?v=tI70-HIc5ro
fn sample_13_tap(uv: vec2<f32>, scale: vec2<f32>) -> vec4<f32> {
	let a = textureSample(org, org_sampler, uv + vec2<f32>(-1.0, -1.0) * scale);
	let b = textureSample(org, org_sampler, uv + vec2<f32>( 0.0, -1.0) * scale);
	let c = textureSample(org, org_sampler, uv + vec2<f32>( 1.0, -1.0) * scale);
	let d = textureSample(org, org_sampler, uv + vec2<f32>(-0.5, -0.5) * scale);
	let e = textureSample(org, org_sampler, uv + vec2<f32>( 0.5, -0.5) * scale);
	let f = textureSample(org, org_sampler, uv + vec2<f32>(-1.0,  0.0) * scale);
	let g = textureSample(org, org_sampler, uv + vec2<f32>( 0.0,  0.0) * scale);
	let h = textureSample(org, org_sampler, uv + vec2<f32>( 1.0,  0.0) * scale);
	let i = textureSample(org, org_sampler, uv + vec2<f32>(-0.5,  0.5) * scale);
	let j = textureSample(org, org_sampler, uv + vec2<f32>( 0.5,  0.5) * scale);
	let k = textureSample(org, org_sampler, uv + vec2<f32>(-1.0,  1.0) * scale);
	let l = textureSample(org, org_sampler, uv + vec2<f32>( 0.0,  1.0) * scale);
	let m = textureSample(org, org_sampler, uv + vec2<f32>( 1.0,  1.0) * scale);

	let div = (1.0 / 4.0) * vec2<f32>(0.5, 0.125);

	var o: vec4<f32> = (d + e + i + j) * div.x;
	o = o + (a + b + g + f) * div.y;
	o = o + (b + c + h + g) * div.y;
	o = o + (f + g + l + k) * div.y;
	o = o + (g + h + m + l) * div.y;

	return o;
}

// samples org using a 3x3 tent filter
//
// NOTE: use a 2x2 filter for better perf, but 3x3 looks better
fn sample_3x3_tent(uv: vec2<f32>, scale: vec2<f32>) -> vec4<f32> {
	let d = vec4<f32>(1.0, 1.0, -1.0, 0.0);

	var s: vec4<f32> = textureSample(org, org_sampler, uv - d.xy * scale);
	s = s + textureSample(org, org_sampler, uv - d.wy * scale) * 2.0;
	s = s + textureSample(org, org_sampler, uv - d.zy * scale);

	s = s + textureSample(org, org_sampler, uv + d.zw * scale) * 2.0;
	s = s + textureSample(org, org_sampler, uv       			 ) * 4.0;
	s = s + textureSample(org, org_sampler, uv + d.xw * scale) * 2.0;

	s = s + textureSample(org, org_sampler, uv + d.zy * scale);
	s = s + textureSample(org, org_sampler, uv + d.wy * scale) * 2.0;
	s = s + textureSample(org, org_sampler, uv + d.xy * scale);

	return s / 16.0;
}

[[stage(fragment)]]
fn down_sample_pre_filter(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let texel_size = 1.0 / vec2<f32>(textureDimensions(org));

	let scale = texel_size;

	let curve = vec3<f32>(
		uniforms.threshold - uniforms.knee,
		uniforms.knee * 2.0,
		0.25 / uniforms.knee,
	);

	var o: vec4<f32> = sample_13_tap(in.uv, scale);

	o = quadratic_threshold(o, uniforms.threshold, curve);
	o = max(o, vec4<f32>(0.00001));

	return o;
}

[[stage(fragment)]]
fn down_sample(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let texel_size = 1.0 / vec2<f32>(textureDimensions(org));

	let scale = texel_size;

	return sample_13_tap(in.uv, scale);
}

[[stage(fragment)]]
fn up_sample(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let texel_size = 1.0 / vec2<f32>(textureDimensions(org));

	let up_sample = sample_3x3_tent(in.uv, texel_size * uniforms.scale);
	var color: vec4<f32> = textureSample(up, org_sampler, in.uv);
	color = vec4<f32>(color.rgb + up_sample.rgb, up_sample.a);

	return color;
}

[[stage(fragment)]]
fn up_sample_final(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let texel_size = 1.0 / vec2<f32>(textureDimensions(org));

	let up_sample = sample_3x3_tent(in.uv, texel_size * uniforms.scale);

	return up_sample;
}
