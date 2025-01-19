#define_import_path bevy_pbr::fast_math

#import bevy_render::maths::{PI, HALF_PI};

// This file includes fast approximations of common irrational and trig functions. These
// are likely most useful when raymarching, for example, where total numeric accuracy can
// be sacrificed for greater sample count.

fn fast_sqrt(x: f32) -> f32 {
    return bitcast<f32>(0x1fbd1df5 + (bitcast<i32>(x) >> 1u));
}

// Slightly less accurate than fast_acos_4, but much simpler.
fn fast_acos(in_x: f32) -> f32 {
    let x = abs(in_x);
    var res = -0.156583 * x + HALF_PI;
    res *= fast_sqrt(1.0 - x);
    return select(PI - res, res, in_x >= 0.0);
}

// 4th order polynomial approximation
// 4 VGRP, 16 ALU Full Rate
// 7 * 10^-5 radians precision
// Reference : Handbook of Mathematical Functions (chapter : Elementary Transcendental Functions), M. Abramowitz and I.A. Stegun, Ed.
fn fast_acos_4(x: f32) -> f32 {
    let x1 = abs(x);
    let x2 = x1 * x1;
    let x3 = x2 * x1;
    var s: f32;

    s = -0.2121144 * x1 + 1.5707288;
    s = 0.0742610 * x2 + s;
    s = -0.0187293 * x3 + s;
    s = fast_sqrt(1.0 - x1) * s;

	// acos function mirroring
    return select(PI - s, s, x >= 0.0);
}

fn fast_atan2(y: f32, x: f32) -> f32 {
    var t0 = max(abs(x), abs(y));
    var t1 = min(abs(x), abs(y));
    var t3 = t1 / t0;
    var t4 = t3 * t3;

    t0 = 0.0872929;
    t0 = t0 * t4 - 0.301895;
    t0 = t0 * t4 + 1.0;
    t3 = t0 * t3;

    t3 = select(t3, (0.5 * PI) - t3, abs(y) > abs(x));
    t3 = select(t3, PI - t3, x < 0);
    t3 = select(-t3, t3, y > 0);

    return t3;
}
