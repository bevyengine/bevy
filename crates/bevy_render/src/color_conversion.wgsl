#define_import_path bevy_render::color_conversion

const RAD2DEG: f32 = 57.295779513; // 180/PI
const DEG2RAD: f32 = 0.01745329252; // PI/180

// Oklaba

fn linear_rgba_to_oklaba(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_oklab(color.xyz), color.w);
}
fn oklaba_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(oklab_to_linear_rgb(color.xyz), color.w);
}

fn linear_rgb_to_oklab(color: vec3<f32>) -> vec3<f32> {
	let l1 = 0.4122214708 * color.x + 0.5363325363 * color.y + 0.0514459929 * color.z;
	let m = 0.2119034982 * color.x + 0.6806995451 * color.y + 0.1073969566 * color.z;
	let s = 0.0883024619 * color.x + 0.2817188376 * color.y + 0.6299787005 * color.z;
	let l_ = pow(l1, 0.33333); // cube root
	let m_ = pow(m, 0.33333); // cube root
	let s_ = pow(s, 0.33333); // cube root
	let l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
	let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
	let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

	return vec3(l, a, b);
}
fn oklab_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let l_ = color.x + 0.3963377774 * color.y + 0.2158037573 * color.z;
	let m_ = color.x - 0.1055613458 * color.y - 0.0638541728 * color.z;
	let s_ = color.x - 0.0894841775 * color.y - 1.2914855480 * color.z;

	let l = l_ * l_ * l_;
	let m = m_ * m_ * m_;
	let s = s_ * s_ * s_;

	let red = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
	let green = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
	let blue = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

	return vec3(red, green, blue);
}

// Srgba

fn linear_rgba_to_srgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_srgb(color.xyz), color.w);
}
fn srgba_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(srgb_to_linear_rgb(color.xyz), color.w);
}

fn srgb_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	return vec3(
		gamma_function(color.x),
		gamma_function(color.y),
		gamma_function(color.z)
	);
}
fn linear_rgb_to_srgb(color: vec3<f32>) -> vec3<f32> {
	return vec3(
		gamma_function_inverse(color.x),
		gamma_function_inverse(color.y),
		gamma_function_inverse(color.z)
	);
}

fn gamma_function(value: f32) -> f32 {
	if value <= 0.0 {
		return value;
	}
	if value <= 0.04045 {
		return value / 12.92; // linear falloff in dark values
	} else {
		return pow((value + 0.055) / 1.055, 2.4); // gamma curve in other area
	}
}
fn gamma_function_inverse(value: f32) -> f32 {
	if value <= 0.0 {
		return value;
	}

	if value <= 0.0031308 {
		return value * 12.92; // linear falloff in dark values
	} else {
		return 1.055 * pow(value, 1.0 / 2.4) - 0.055; // gamma curve in other area
	}
}

// Xyza

/// [D65 White Point](https://en.wikipedia.org/wiki/Illuminant_D65#Definition)
const XYZ_D65_WHITE: vec3<f32> = vec3(0.95047, 1.0, 1.08883);

fn linear_rgba_to_xyza(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_xyz(color.xyz), color.w);
}
fn xyza_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(xyz_to_linear_rgb(color.xyz), color.w);
}

fn linear_rgb_to_xyz(color: vec3<f32>) -> vec3<f32> {
	let x = color.x * 0.4124564 + color.y * 0.3575761 + color.z * 0.1804375;
	let y = color.x * 0.2126729 + color.y * 0.7151522 + color.z * 0.072175;
	let z = color.x * 0.0193339 + color.y * 0.119192 + color.z * 0.9503041;

	return vec3(x, y, z);
}
fn xyz_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let r = color.x * 3.2404542 + color.y * -1.5371385 + color.z * -0.4985314;
	let g = color.x * -0.969266 + color.y * 1.8760108 + color.z * 0.041556;
	let b = color.x * 0.0556434 + color.y * -0.2040259 + color.z * 1.0572252;

	return vec3(r, g, b);
}

// Oklcha

// LinearRgba <-> Oklcha
fn linear_rgba_to_oklcha(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_oklch(color.xyz), color.w);
}
fn oklcha_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(oklch_to_linear_rgb(color.xyz), color.w);
}

fn linear_rgb_to_oklch(color: vec3<f32>) -> vec3<f32> {
	let oklab = linear_rgb_to_oklab(color);
	return oklab_to_oklch(oklab);
}
fn oklch_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let oklab = oklch_to_oklab(color);
	return oklab_to_linear_rgb(oklab);
}

// Oklaba <-> Oklcha
fn oklaba_to_oklcha(color: vec4<f32>) -> vec4<f32> {
	return vec4(oklab_to_oklch(color.xyz), color.w);
}
fn oklcha_to_oklaba(color: vec4<f32>) -> vec4<f32> {
	return vec4(oklch_to_oklab(color.xyz), color.w);
}

fn oklab_to_oklch(color: vec3<f32>) -> vec3<f32> {
	let chroma = sqrt(color.y * color.y + color.z * color.z);
	var hue = atan2(color.z, color.y) * RAD2DEG;

	if hue < 0.0 {
		hue += 360.0;
	}

	return vec3(color.x, chroma, hue);
}
fn oklch_to_oklab(color: vec3<f32>) -> vec3<f32> {
	let a = color.y * cos(color.z * DEG2RAD);
	let b = color.y * sin(color.z * DEG2RAD);

	return vec3(color.x, a, b);
}

// Laba

/// CIE Epsilon Constant
///
/// See [Continuity (16) (17)](http://brucelindbloom.com/index.html?LContinuity.html)
const LAB_CIE_EPSILON: f32 = 216.0 / 24389.0;
/// CIE Kappa Constant
///
/// See [Continuity (16) (17)](http://brucelindbloom.com/index.html?LContinuity.html)
const LAB_CIE_KAPPA: f32 = 24389.0 / 27.0;

// LinearRgba <-> Laba
fn laba_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(lab_to_linear_rgb(color.xyz), color.w);
}
fn linear_rgba_to_laba(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_lab(color.xyz), color.w);
}

fn linear_rgb_to_lab(color: vec3<f32>) -> vec3<f32> {
	let xyz = linear_rgb_to_xyz(color);
	return xyz_to_lab(xyz);
}
fn lab_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let xyz = lab_to_xyz(color);
	return xyz_to_linear_rgb(xyz);
}


// Xyza <-> Laba
fn laba_to_xyza(color: vec4<f32>) -> vec4<f32> {
	return vec4(lab_to_xyz(color.xyz), color.w);
}
fn xyza_to_laba(color: vec4<f32>) -> vec4<f32> {
	return vec4(xyz_to_lab(color.xyz), color.w);
}

fn lab_to_xyz(color: vec3<f32>) -> vec3<f32> {
	// Based on http://www.brucelindbloom.com/index.html?Eqn_Lab_to_XYZ.html
	let l = 100. * color.x;
	let a = 100. * color.y;
	let b = 100. * color.z;

	let fy = (l + 16.0) / 116.0;
	let fx = a / 500.0 + fy;
	let fz = fy - b / 200.0;

	let fx3 = fx * fx * fx;
	var xr: f32;
	if fx3 > LAB_CIE_EPSILON {
		xr = fx3;
	} else {
		xr = (116.0 * fx - 16.0) / LAB_CIE_KAPPA;
	}

	var yr: f32;
	if l > LAB_CIE_EPSILON * LAB_CIE_KAPPA {
		yr = pow((l + 16.0) / 116.0, 3.0);
	} else {
		yr = l / LAB_CIE_KAPPA;
	}

	let fz3 = fz * fz * fz;
	var zr: f32;
	if fz3 > LAB_CIE_EPSILON {
		zr = fz3;
	} else {
		zr = (116.0 * fz - 16.0) / LAB_CIE_KAPPA;
	}

	let x = xr * XYZ_D65_WHITE.x;
	let y = yr * XYZ_D65_WHITE.y;
	let z = zr * XYZ_D65_WHITE.z;

	return vec3(x, y, z);
}
fn xyz_to_lab(color: vec3<f32>) -> vec3<f32> {
	// Based on http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_Lab.html
	let xr = color.x / XYZ_D65_WHITE.x;
	let yr = color.y / XYZ_D65_WHITE.y;
	let zr = color.z / XYZ_D65_WHITE.z;

	var fx: f32;
	if xr > LAB_CIE_EPSILON {
		fx = pow(xr, 0.33333);
	} else {
		fx = (LAB_CIE_KAPPA * xr + 16.0) / 116.0;
	}

	var fy: f32;
	if yr > LAB_CIE_EPSILON {
		fy = pow(yr, 0.33333);
	} else {
		fy = (LAB_CIE_KAPPA * yr + 16.0) / 116.0;
	};

	var fz: f32;
	if yr > LAB_CIE_EPSILON {
		fz = pow(zr, 0.33333);
	} else {
		fz = (LAB_CIE_KAPPA * zr + 16.0) / 116.0;
	};
	
	let l = 1.16 * fy - 0.16;
	let a = 5.00 * (fx - fy);
	let b = 2.00 * (fy - fz);

	return vec3(l, a, b);
}

// Lcha

// Laba <-> Lcha
fn laba_to_lcha(color: vec4<f32>) -> vec4<f32> {
	return vec4(lab_to_lch(color.xyz), color.w);
}
fn lcha_to_laba(color: vec4<f32>) -> vec4<f32> {
	return vec4(lch_to_lab(color.xyz), color.w);
}

fn lab_to_lch(color: vec3<f32>) -> vec3<f32> {
	let c = clamp(sqrt(color.y * color.y + color.z * color.z), 0.0, 1.5);
	var h = atan2(color.z * DEG2RAD, color.y * DEG2RAD) * RAD2DEG;
	if h < 0.0 {
		h += 360.0;
	}

	return vec3(color.x, c, h);
}
fn lch_to_lab(color: vec3<f32>) -> vec3<f32> {
	let a = color.y * cos(color.z * DEG2RAD);
	let b = color.y * sin(color.z * DEG2RAD);

	return vec3(color.x, a, b);
}

// Xyza <-> Lcha
fn lcha_to_xyza(color: vec4<f32>) -> vec4<f32> {
	return vec4(lch_to_xyz(color.xyz), color.w);
}
fn xyza_to_lcha(color: vec4<f32>) -> vec4<f32> {
	return vec4(xyz_to_lch(color.xyz), color.w);
}

fn xyz_to_lch(color: vec3<f32>) -> vec3<f32> {
	let lab = xyz_to_lab(color);
	return lab_to_lch(lab);
}
fn lch_to_xyz(color: vec3<f32>) -> vec3<f32> {
	let lab = lch_to_lab(color);
	return lab_to_xyz(lab);
}

// LinearRgba <-> Lcha
fn lcha_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(lch_to_linear_rgb(color.xyz), color.w);
}
fn linear_rgba_to_lcha(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_lch(color.xyz), color.w);
}

fn linear_rgb_to_lch(color: vec3<f32>) -> vec3<f32> {
	let xyz = linear_rgb_to_xyz(color);
	return xyz_to_lch(xyz);
}
fn lch_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let xyz = lch_to_xyz(color);
	return xyz_to_linear_rgb(xyz);
}

// Hwba

// Srgba <-> Hwba
fn hwba_to_srgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hwb_to_srgb(color.xyz), color.w);
}
fn srgba_to_hwba(color: vec4<f32>) -> vec4<f32> {
	return vec4(srgb_to_hwb(color.xyz), color.w);
}

fn srgb_to_hwb(color: vec3<f32>) -> vec3<f32> {
	// Based on "HWB - A More Intuitive Hue-Based Color Model" Appendix B
	let x_max = max(max(max(0.0, color.x), color.y), color.z);
	let x_min = min(min(min(1.0, color.x), color.y), color.z);

	let chroma = x_max - x_min;

	var hue: f32;
	if chroma == 0.0 {
		hue = 0.0;
	} else if color.x == x_max {
		hue = 60.0 * (color.y - color.z) / chroma;
	} else if color.y == x_max {
		hue = 60.0 * (2.0 + (color.z - color.x) / chroma);
	} else {
		hue = 60.0 * (4.0 + (color.x - color.y) / chroma);
	};

	if hue < 0.0 { 
		hue += 360.0;
	}

	let whiteness = x_min;
	let blackness = 1.0 - x_max;

	return vec3(hue, whiteness, blackness);
}
fn hwb_to_srgb(color: vec3<f32>) -> vec3<f32> {
	// Based on "HWB - A More Intuitive Hue-Based Color Model" Appendix B
	let w = color.y;
	let v = 1. - color.z;

	let h = (color.x % 360.) / 60.;
	let i = floor(h);
	
	var f = h - i;
	if i % 2.0 == 1 {
		f = 1.0 - f;
	}

	let n = w + f * (v - w);

	var srgb: vec3<f32>;
	if i < 1.0 {
		srgb = vec3(v, n, w);
	} else if i < 2.0 {
		srgb = vec3(n, v, w);
	} else if i < 3.0 {
		srgb = vec3(w, v, n);
	} else if i < 4.0 {
		srgb = vec3(w, n, v);
	} else if i < 5.0 {
		srgb = vec3(n, w, v);
	} else {
		srgb = vec3(v, w, n);
	}
	
	return srgb;
}

// LinearRgba <-> Hwba
fn hwba_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hwb_to_linear_rgb(color.xyz), color.w);
}
fn linear_rgba_to_hwba(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_hwb(color.xyz), color.w);
}

fn linear_rgb_to_hwb(color: vec3<f32>) -> vec3<f32> {
	let srgb = linear_rgb_to_srgb(color);
	return srgb_to_hwb(srgb);
}
fn hwb_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let srgb = hwb_to_srgb(color);
	return srgb_to_linear_rgb(srgb);
}

// Hsva

// Hwba <-> Hsva
fn hwba_to_hsva(color: vec4<f32>) -> vec4<f32> {
	return vec4(hwb_to_hsv(color.xyz), color.w);
}
fn hsva_to_hwba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsv_to_hwb(color.xyz), color.w);
}

fn hsv_to_hwb(color: vec3<f32>) -> vec3<f32> {
	// Based on https://en.wikipedia.org/wiki/HWB_color_model#Conversion
	let whiteness = (1. - color.y) * color.z;
	let blackness = 1. - color.z;

	return vec3(color.x, whiteness, blackness);
}
fn hwb_to_hsv(color: vec3<f32>) -> vec3<f32> {
	// Based on https://en.wikipedia.org/wiki/HWB_color_model#Conversion
	let value = 1. - color.z;
	let saturation = 1. - (color.y / value);

	return vec3(color.x, saturation, value);
}

// Srgba <-> Hsva
fn hsva_to_srgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsv_to_srgb(color.xyz), color.w);
}
fn srgba_to_hsva(color: vec4<f32>) -> vec4<f32> {
	return vec4(srgb_to_hsv(color.xyz), color.w);
}

fn srgb_to_hsv(color: vec3<f32>) -> vec3<f32> {
	let hwb = srgb_to_hwb(color);
	return hwb_to_hsv(hwb);
}
fn hsv_to_srgb(color: vec3<f32>) -> vec3<f32> {
	let hwb = hsv_to_hwb(color);
	return hwb_to_srgb(hwb);
}

// LinearRgba <-> Hsva
fn hsva_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsv_to_linear_rgb(color.xyz), color.w);
}
fn linear_rgba_to_hsva(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_hsv(color.xyz), color.w);
}

fn linear_rgb_to_hsv(color: vec3<f32>) -> vec3<f32> {
	let srgb = linear_rgb_to_srgb(color);
	return srgb_to_hsv(srgb);
}
fn hsv_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let srgb = hsv_to_srgb(color);
	return srgb_to_linear_rgb(srgb);
}

// Hsla

// Hsva <-> Hsla
fn hsla_to_hsva(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsl_to_hsv(color.xyz), color.w);
}
fn hsva_to_hsla(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsv_to_hsl(color.xyz), color.w);
}

fn hsv_to_hsl(color: vec3<f32>) -> vec3<f32> {
	// Based on https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_HSL
	let lightness = color.z * (1. - color.y / 2.);
	var saturation: f32;
	if lightness == 0. || lightness == 1. {
		saturation = 0.0;
	} else {
		saturation = (color.z - lightness) / min(lightness, 1. - lightness);
	};

	return vec3(color.x, saturation, lightness);
}
fn hsl_to_hsv(color: vec3<f32>) -> vec3<f32> {
	// Based on https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_HSV
	let value = color.z + color.y * min(color.z, 1. - color.z);
	var saturation: f32;
	if value == 0.0 {
		saturation = 0.0;
	} else {
		saturation = 2. * (1. - (color.z / value));
	}

	return vec3(color.x, saturation, value);
}

// Hwba <-> Hsla
fn hsla_to_hwba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsl_to_hwb(color.xyz), color.w);
}
fn hwba_to_hsla(color: vec4<f32>) -> vec4<f32> {
	return vec4(hwb_to_hsl(color.xyz), color.w);
}

fn hwb_to_hsl(color: vec3<f32>) -> vec3<f32> {
	let hsv = hwb_to_hsv(color);
	return hsv_to_hsl(hsv);
}
fn hsl_to_hwb(color: vec3<f32>) -> vec3<f32> {
	let hsv = hsl_to_hsv(color);
	return hsv_to_hwb(hsv);
}

// Srgba <-> Hsla
fn hsla_to_srgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsl_to_srgb(color.xyz), color.w);
}
fn srgba_to_hsla(color: vec4<f32>) -> vec4<f32> {
	return vec4(srgb_to_hsl(color.xyz), color.w);
}

fn srgb_to_hsl(color: vec3<f32>) -> vec3<f32> {
	let hwb = srgb_to_hwb(color);
	return hwb_to_hsl(hwb);
}
fn hsl_to_srgb(color: vec3<f32>) -> vec3<f32> {
	let hwb = hsl_to_hwb(color);
	return hwb_to_srgb(hwb);
}

// LinearRgba <-> Hsla
fn hsla_to_linear_rgba(color: vec4<f32>) -> vec4<f32> {
	return vec4(hsl_to_linear_rgb(color.xyz), color.w);
}
fn linear_rgba_to_hsla(color: vec4<f32>) -> vec4<f32> {
	return vec4(linear_rgb_to_hsl(color.xyz), color.w);
}

fn linear_rgb_to_hsl(color: vec3<f32>) -> vec3<f32> {
	let srgb = linear_rgb_to_srgb(color);
	return srgb_to_hsl(srgb);
}
fn hsl_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
	let srgb = hsl_to_srgb(color);
	return srgb_to_linear_rgb(srgb);
}
