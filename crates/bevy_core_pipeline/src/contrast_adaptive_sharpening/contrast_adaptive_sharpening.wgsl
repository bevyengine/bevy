// From https://github.com/CeeJayDK/SweetFX/blob/master/Shaders/CAS.fx
//
// LICENSE
// =======
// Copyright (c) 2017-2019 Advanced Micro Devices, Inc. All rights reserved.
// -------
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation
// files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
// -------
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
// Software.
// -------
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
// WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE

//Initial port to ReShade: SLSNe	https://gist.github.com/SLSNe/bbaf2d77db0b2a2a0755df581b3cf00c
//Optimizations by Marty McFly:
//	vectorized math, even with scalar gcn hardware this should work
//	out the same, order of operations has not changed
//	For some reason, it went from 64 to 48 instructions, a lot of MOV gone
//	Also modified the way the final window is calculated
//	  
//	reordered min() and max() operations, from 11 down to 9 registers	
//
//	restructured final weighting, 49 -> 48 instructions
//
//	delayed RCP to replace SQRT with RSQRT
//
//	removed the saturate() from the control var as it is clamped
//	by UI manager already, 48 -> 47 instructions
//
//	replaced tex2D with tex2Doffset intrinsic (address offset by immediate integer)
//	47 -> 43 instructions
//	9 -> 8 registers
//Further modified by OopyDoopy and Lord of Lunacy:
//	Changed wording in the UI for the existing variable and added a new variable and relevant code to adjust sharpening strength.
//Fix by Lord of Lunacy:
//	Made the shader use a linear colorspace rather than sRGB, as recommended by the original AMD documentation from FidelityFX.
//Modified by CeeJay.dk:
//	Included a label and tooltip description. I followed AMDs official naming guidelines for FidelityFX.
//
//	Used gather trick to reduce the number of texture operations by one (9 -> 8). It's now 42 -> 51 instructions but still faster
//	because of the texture operation that was optimized away.
//Fix by CeeJay.dk
//	Fixed precision issues with the gather at super high resolutions
//	Also tried to refactor the samples so more work can be done while they are being sampled, but it's not so easy and the gains
//	I'm seeing are so small they might be statistical noise. So it MIGHT be faster - no promises.
//Ported to Bevy wgsl by Elabajaba

#import bevy_core_pipeline::fullscreen_vertex_shader

struct CASUniforms {
	contrast_adaption: f32,
	sharpening_intensity: f32,
};

@group(0) @binding(0)
var screenTexture: texture_2d<f32>;
@group(0) @binding(1)
var samp: sampler;
@group(0) @binding(2)
var<uniform> uniforms: CASUniforms;

// Performs Contrast Adaptive Sharpening (CAS) post-process sharpening.
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
	// fetch a 3x3 neighborhood around the pixel 'e',
	//  a b c
	//  d(e)f
	//  g h i
	
	let a = textureSample(screenTexture, samp, in.uv, vec2<i32>(-1, -1)).rgb;
	let b = textureSample(screenTexture, samp, in.uv, vec2<i32>(0, -1)).rgb;
	let c = textureSample(screenTexture, samp, in.uv, vec2<i32>(1, -1)).rgb;
	let d = textureSample(screenTexture, samp, in.uv, vec2<i32>(-1, 0)).rgb;
	// We need the alpha value of the pixel we're working on for the output
	let e = textureSample(screenTexture, samp, in.uv).rgbw;
	let f = textureSample(screenTexture, samp, in.uv, vec2<i32>(1, 0)).rgb;
	let g = textureSample(screenTexture, samp, in.uv, vec2<i32>(-1, 1)).rgb;
	let h = textureSample(screenTexture, samp, in.uv, vec2<i32>(0, 1)).rgb;
	let i = textureSample(screenTexture, samp, in.uv, vec2<i32>(1, 1)).rgb;

	// Soft min and max.
	//  a b c			  b
	//  d e f * 0.5  +  d e f * 0.5
	//  g h i			  h
	// These are 2.0x bigger (factored out the extra multiply).
	var mnRGB = min(min(min(d, e.rgb), min(f, b)), h);
	let mnRGB2 = min(mnRGB, min(min(a, c), min(g, i)));
	mnRGB += mnRGB2;

	var mxRGB = max(max(max(d, e.rgb), max(f, b)), h);
	let mxRGB2 = max(mxRGB, max(max(a, c), max(g, i)));
	mxRGB += mxRGB2;

	// Smooth minimum distance to signal limit divided by smooth max.
	let rcpMRGB = 1.0 / mxRGB;
	var ampRGB = saturate(min(mnRGB, 2.0 - mxRGB) * rcpMRGB);

	// Shaping amount of sharpening.
	ampRGB = inverseSqrt(ampRGB);
	let peak = -3.0 * uniforms.contrast_adaption + 8.0;
	let wRGB = -1.0 / (ampRGB * peak);
	let rcpWeightRGB = 1.0 / (4.0 * wRGB + 1.0);

	//						  0 w 0
	//  Filter shape:		  w 1 w
	//						  0 w 0  
	let window = (b + d) + (f + h);
	let outColor = saturate((window * wRGB + e.rgb) * rcpWeightRGB);

	let out = mix(e.rgb, outColor, uniforms.sharpening_intensity);

	return vec4<f32>(out, e.w);
}