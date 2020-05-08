// pathfinder/c/src/lib.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! C bindings to Pathfinder.

use font_kit::handle::Handle;
use foreign_types::ForeignTypeRef;
use gl;
use pathfinder_canvas::{Canvas, CanvasFontContext, CanvasRenderingContext2D, FillStyle, LineJoin};
use pathfinder_canvas::{Path2D, TextAlign, TextMetrics};
use pathfinder_color::{ColorF, ColorU};
use pathfinder_content::fill::FillRule;
use pathfinder_content::outline::ArcDirection;
use pathfinder_content::stroke::LineCap;
use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::transform2d::{Matrix2x2F, Transform2F};
use pathfinder_geometry::transform3d::{Perspective, Transform4F};
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::{BuildOptions, RenderTransform};
use pathfinder_renderer::scene::Scene;
use pathfinder_simd::default::F32x4;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::slice;
use std::str;

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use metal::{CAMetalLayer, CoreAnimationLayerRef};
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use pathfinder_metal::MetalDevice;

// Constants

// `canvas`

pub const PF_LINE_CAP_BUTT:     u8 = 0;
pub const PF_LINE_CAP_SQUARE:   u8 = 1;
pub const PF_LINE_CAP_ROUND:    u8 = 2;

pub const PF_LINE_JOIN_MITER:   u8 = 0;
pub const PF_LINE_JOIN_BEVEL:   u8 = 1;
pub const PF_LINE_JOIN_ROUND:   u8 = 2;

pub const PF_TEXT_ALIGN_LEFT:   u8 = 0;
pub const PF_TEXT_ALIGN_CENTER: u8 = 1;
pub const PF_TEXT_ALIGN_RIGHT:  u8 = 2;

// `content`

pub const PF_ARC_DIRECTION_CW:  u8 = 0;
pub const PF_ARC_DIRECTION_CCW: u8 = 1;

// `gl`

pub const PF_GL_VERSION_GL3:    u8 = 0;
pub const PF_GL_VERSION_GLES3:  u8 = 1;

// `renderer`

pub const PF_RENDERER_OPTIONS_FLAGS_HAS_BACKGROUND_COLOR: u8 = 0x1;

// Types

// External: `font-kit`
pub type FKHandleRef = *mut Handle;

// `canvas`
pub type PFCanvasRef = *mut CanvasRenderingContext2D;
pub type PFPathRef = *mut Path2D;
pub type PFCanvasFontContextRef = *mut CanvasFontContext;
pub type PFFillStyleRef = *mut FillStyle;
pub type PFLineCap = u8;
pub type PFLineJoin = u8;
pub type PFArcDirection = u8;
pub type PFTextAlign = u8;
#[repr(C)]
pub struct PFTextMetrics {
    pub width: f32,
}

// `content`
#[repr(C)]
pub struct PFColorF {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
#[repr(C)]
pub struct PFColorU {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

// `geometry`
#[repr(C)]
pub struct PFVector2F {
    pub x: f32,
    pub y: f32,
}
#[repr(C)]
pub struct PFVector2I {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
pub struct PFRectF {
    pub origin: PFVector2F,
    pub lower_right: PFVector2F,
}
#[repr(C)]
pub struct PFRectI {
    pub origin: PFVector2I,
    pub lower_right: PFVector2I,
}
/// Row-major order.
#[repr(C)]
pub struct PFMatrix2x2F {
    pub m00: f32, pub m01: f32,
    pub m10: f32, pub m11: f32,
}
/// Row-major order.
#[repr(C)]
pub struct PFTransform2F {
    pub matrix: PFMatrix2x2F,
    pub vector: PFVector2F,
}
/// Row-major order.
#[repr(C)]
pub struct PFTransform4F {
    pub m00: f32, pub m01: f32, pub m02: f32, pub m03: f32,
    pub m10: f32, pub m11: f32, pub m12: f32, pub m13: f32,
    pub m20: f32, pub m21: f32, pub m22: f32, pub m23: f32,
    pub m30: f32, pub m31: f32, pub m32: f32, pub m33: f32,
}
#[repr(C)]
pub struct PFPerspective {
    pub transform: PFTransform4F,
    pub window_size: PFVector2I,
}

// `gl`
pub type PFGLDeviceRef = *mut GLDevice;
pub type PFGLVersion = u8;
pub type PFGLFunctionLoader = extern "C" fn(name: *const c_char, userdata: *mut c_void)
                                            -> *const c_void;
// `gpu`
pub type PFGLDestFramebufferRef = *mut DestFramebuffer<GLDevice>;
pub type PFGLRendererRef = *mut Renderer<GLDevice>;
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
pub type PFMetalDestFramebufferRef = *mut DestFramebuffer<MetalDevice>;
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
pub type PFMetalRendererRef = *mut Renderer<MetalDevice>;
// FIXME(pcwalton): Double-boxing is unfortunate. Remove this when `std::raw::TraitObject` is
// stable?
pub type PFResourceLoaderRef = *mut ResourceLoaderWrapper;
pub struct ResourceLoaderWrapper(Box<dyn ResourceLoader>);

// `metal`
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
pub type PFMetalDeviceRef = *mut MetalDevice;

// `renderer`
pub type PFSceneRef = *mut Scene;
pub type PFSceneProxyRef = *mut SceneProxy;
#[repr(C)]
pub struct PFRendererOptions {
    pub background_color: PFColorF,
    pub flags: PFRendererOptionsFlags,
}
pub type PFRendererOptionsFlags = u8;
pub type PFBuildOptionsRef = *mut BuildOptions;
pub type PFRenderTransformRef = *mut RenderTransform;

// `canvas`

/// This function internally adds a reference to the font context. Therefore, if you created the
/// font context, you must release it yourself to avoid a leak.
#[no_mangle]
pub unsafe extern "C" fn PFCanvasCreate(font_context: PFCanvasFontContextRef,
                                        size: *const PFVector2F)
                                        -> PFCanvasRef {
    Box::into_raw(Box::new(Canvas::new((*size).to_rust()).get_context_2d((*font_context).clone())))
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasDestroy(canvas: PFCanvasRef) {
    drop(Box::from_raw(canvas))
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasFontContextCreateWithSystemSource() -> PFCanvasFontContextRef {
    Box::into_raw(Box::new(CanvasFontContext::from_system_source()))
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasFontContextCreateWithFonts(fonts: *const FKHandleRef,
                                                            font_count: usize)
                                                            -> PFCanvasFontContextRef {
    let fonts = slice::from_raw_parts(fonts, font_count);
    Box::into_raw(Box::new(CanvasFontContext::from_fonts(fonts.into_iter().map(|font| {
        (**font).clone()
    }))))
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasFontContextAddRef(font_context: PFCanvasFontContextRef)
                                                   -> PFCanvasFontContextRef {
    Box::into_raw(Box::new((*font_context).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasFontContextRelease(font_context: PFCanvasFontContextRef) {
    drop(Box::from_raw(font_context))
}

/// This function takes ownership of the supplied canvas and will automatically destroy it when
/// the scene is destroyed.
#[no_mangle]
pub unsafe extern "C" fn PFCanvasCreateScene(canvas: PFCanvasRef) -> PFSceneRef {
    Box::into_raw(Box::new(Box::from_raw(canvas).into_canvas().into_scene()))
}

// Drawing rectangles

#[no_mangle]
pub unsafe extern "C" fn PFCanvasFillRect(canvas: PFCanvasRef, rect: *const PFRectF) {
    (*canvas).fill_rect((*rect).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasStrokeRect(canvas: PFCanvasRef, rect: *const PFRectF) {
    (*canvas).stroke_rect((*rect).to_rust())
}

// Drawing text

#[no_mangle]
pub unsafe extern "C" fn PFCanvasFillText(canvas: PFCanvasRef,
                                          string: *const c_char,
                                          string_len: usize,
                                          position: *const PFVector2F) {
    (*canvas).fill_text(to_rust_string(&string, string_len), (*position).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasStrokeText(canvas: PFCanvasRef,
                                            string: *const c_char,
                                            string_len: usize,
                                            position: *const PFVector2F) {
    (*canvas).stroke_text(to_rust_string(&string, string_len), (*position).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasMeasureText(canvas: PFCanvasRef,
                                             string: *const c_char,
                                             string_len: usize,
                                             out_text_metrics: *mut PFTextMetrics) {
    debug_assert!(!out_text_metrics.is_null());
    *out_text_metrics = (*canvas).measure_text(to_rust_string(&string, string_len)).to_c()
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetLineWidth(canvas: PFCanvasRef, new_line_width: f32) {
    (*canvas).set_line_width(new_line_width)
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetLineCap(canvas: PFCanvasRef, new_line_cap: PFLineCap) {
    (*canvas).set_line_cap(match new_line_cap {
        PF_LINE_CAP_SQUARE => LineCap::Square,
        PF_LINE_CAP_ROUND  => LineCap::Round,
        _                  => LineCap::Butt,
    });
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetLineJoin(canvas: PFCanvasRef, new_line_join: PFLineJoin) {
    (*canvas).set_line_join(match new_line_join {
        PF_LINE_JOIN_BEVEL => LineJoin::Bevel,
        PF_LINE_JOIN_ROUND => LineJoin::Round,
        _                  => LineJoin::Miter,
    });
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetMiterLimit(canvas: PFCanvasRef, new_miter_limit: f32) {
    (*canvas).set_miter_limit(new_miter_limit);
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetLineDash(canvas: PFCanvasRef,
                                             new_line_dashes: *const f32,
                                             new_line_dash_count: usize) {
    (*canvas).set_line_dash(slice::from_raw_parts(new_line_dashes, new_line_dash_count).to_vec())
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetTransform(canvas: PFCanvasRef,
                                              transform: *const PFTransform2F) {
    (*canvas).set_transform(&(*transform).to_rust());
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasResetTransform(canvas: PFCanvasRef) {
    (*canvas).reset_transform();
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSave(canvas: PFCanvasRef) {
    (*canvas).save();
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasRestore(canvas: PFCanvasRef) {
    (*canvas).restore();
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetLineDashOffset(canvas: PFCanvasRef, new_offset: f32) {
    (*canvas).set_line_dash_offset(new_offset)
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetFontByPostScriptName(canvas: PFCanvasRef,
                                                         postscript_name: *const c_char,
                                                         postscript_name_len: usize) {
    (*canvas).set_font(to_rust_string(&postscript_name, postscript_name_len))
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetFontSize(canvas: PFCanvasRef, new_font_size: f32) {
    (*canvas).set_font_size(new_font_size)
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetTextAlign(canvas: PFCanvasRef, new_text_align: PFTextAlign) {
    (*canvas).set_text_align(match new_text_align {
        PF_TEXT_ALIGN_CENTER => TextAlign::Center,
        PF_TEXT_ALIGN_RIGHT  => TextAlign::Right,
        _                    => TextAlign::Left,
    });
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetFillStyle(canvas: PFCanvasRef, fill_style: PFFillStyleRef) {
    // FIXME(pcwalton): Avoid the copy?
    (*canvas).set_fill_style((*fill_style).clone())
}

#[no_mangle]
pub unsafe extern "C" fn PFCanvasSetStrokeStyle(canvas: PFCanvasRef,
                                                stroke_style: PFFillStyleRef) {
    // FIXME(pcwalton): Avoid the copy?
    (*canvas).set_stroke_style((*stroke_style).clone())
}

/// This function automatically destroys the path. If you wish to use the path again, clone it
/// first.
#[no_mangle]
pub unsafe extern "C" fn PFCanvasFillPath(canvas: PFCanvasRef, path: PFPathRef) {
    // TODO(pcwalton): Expose fill rules to the C API.
    (*canvas).fill_path(*Box::from_raw(path), FillRule::Winding)
}

/// This function automatically destroys the path. If you wish to use the path again, clone it
/// first.
#[no_mangle]
pub unsafe extern "C" fn PFCanvasStrokePath(canvas: PFCanvasRef, path: PFPathRef) {
    (*canvas).stroke_path(*Box::from_raw(path))
}

#[no_mangle]
pub unsafe extern "C" fn PFPathCreate() -> PFPathRef {
    Box::into_raw(Box::new(Path2D::new()))
}

#[no_mangle]
pub unsafe extern "C" fn PFPathDestroy(path: PFPathRef) {
    drop(Box::from_raw(path))
}

#[no_mangle]
pub unsafe extern "C" fn PFPathClone(path: PFPathRef) -> PFPathRef {
    Box::into_raw(Box::new((*path).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn PFPathMoveTo(path: PFPathRef, to: *const PFVector2F) {
    (*path).move_to((*to).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFPathLineTo(path: PFPathRef, to: *const PFVector2F) {
    (*path).line_to((*to).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFPathQuadraticCurveTo(path: PFPathRef,
                                                ctrl: *const PFVector2F,
                                                to: *const PFVector2F) {
    (*path).quadratic_curve_to((*ctrl).to_rust(), (*to).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFPathBezierCurveTo(path: PFPathRef,
                                             ctrl0: *const PFVector2F,
                                             ctrl1: *const PFVector2F,
                                             to: *const PFVector2F) {
    (*path).bezier_curve_to((*ctrl0).to_rust(), (*ctrl1).to_rust(), (*to).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFPathArc(path: PFPathRef,
                                   center: *const PFVector2F,
                                   radius: f32,
                                   start_angle: f32,
                                   end_angle: f32,
                                   direction: PFArcDirection) {
    let direction = if direction == 0 { ArcDirection::CW } else { ArcDirection::CCW };
    (*path).arc((*center).to_rust(), radius, start_angle, end_angle, direction)
}

#[no_mangle]
pub unsafe extern "C" fn PFPathArcTo(path: PFPathRef,
                                     ctrl: *const PFVector2F,
                                     to: *const PFVector2F,
                                     radius: f32) {
    (*path).arc_to((*ctrl).to_rust(), (*to).to_rust(), radius)
}

#[no_mangle]
pub unsafe extern "C" fn PFPathRect(path: PFPathRef, rect: *const PFRectF) {
    (*path).rect((*rect).to_rust())
}

#[no_mangle]
pub unsafe extern "C" fn PFPathEllipse(path: PFPathRef,
                                       center: *const PFVector2F,
                                       axes: *const PFVector2F,
                                       rotation: f32,
                                       start_angle: f32,
                                       end_angle: f32) {
    (*path).ellipse((*center).to_rust(), (*axes).to_rust(), rotation, start_angle, end_angle)
}

#[no_mangle]
pub unsafe extern "C" fn PFPathClosePath(path: PFPathRef) {
    (*path).close_path()
}

#[no_mangle]
pub unsafe extern "C" fn PFFillStyleCreateColor(color: *const PFColorU) -> PFFillStyleRef {
    Box::into_raw(Box::new(FillStyle::Color((*color).to_rust())))
}

#[no_mangle]
pub unsafe extern "C" fn PFFillStyleDestroy(fill_style: PFFillStyleRef) {
    drop(Box::from_raw(fill_style))
}

// `gl`

#[no_mangle]
pub unsafe extern "C" fn PFFilesystemResourceLoaderLocate() -> PFResourceLoaderRef {
    let loader = Box::new(FilesystemResourceLoader::locate());
    Box::into_raw(Box::new(ResourceLoaderWrapper(loader as Box<dyn ResourceLoader>)))
}

#[no_mangle]
pub unsafe extern "C" fn PFGLLoadWith(loader: PFGLFunctionLoader, userdata: *mut c_void) {
    gl::load_with(|name| {
        let name = CString::new(name).unwrap();
        loader(name.as_ptr(), userdata)
    });
}

#[no_mangle]
pub unsafe extern "C" fn PFGLDeviceCreate(version: PFGLVersion, default_framebuffer: u32)
                                          -> PFGLDeviceRef {
    let version = match version { PF_GL_VERSION_GLES3 => GLVersion::GLES3, _ => GLVersion::GL3 };
    Box::into_raw(Box::new(GLDevice::new(version, default_framebuffer)))
}

#[no_mangle]
pub unsafe extern "C" fn PFGLDeviceDestroy(device: PFGLDeviceRef) {
    drop(Box::from_raw(device))
}

#[no_mangle]
pub unsafe extern "C" fn PFResourceLoaderDestroy(loader: PFResourceLoaderRef) {
    drop(Box::from_raw(loader))
}

// `gpu`

#[no_mangle]
pub unsafe extern "C" fn PFGLDestFramebufferCreateFullWindow(window_size: *const PFVector2I)
                                                             -> PFGLDestFramebufferRef {
    Box::into_raw(Box::new(DestFramebuffer::full_window((*window_size).to_rust())))
}

#[no_mangle]
pub unsafe extern "C" fn PFGLDestFramebufferDestroy(dest_framebuffer: PFGLDestFramebufferRef) {
    drop(Box::from_raw(dest_framebuffer))
}

/// This function takes ownership of and automatically takes responsibility for destroying `device`
/// and `dest_framebuffer`. However, it does not take ownership of `resources`; therefore, if you
/// created the resource loader, you must destroy it yourself to avoid a memory leak.
#[no_mangle]
pub unsafe extern "C" fn PFGLRendererCreate(device: PFGLDeviceRef,
                                            resources: PFResourceLoaderRef,
                                            dest_framebuffer: PFGLDestFramebufferRef,
                                            options: *const PFRendererOptions)
                                            -> PFGLRendererRef {
    Box::into_raw(Box::new(Renderer::new(*Box::from_raw(device),
                                         &*((*resources).0),
                                         *Box::from_raw(dest_framebuffer),
                                         (*options).to_rust())))
}

#[no_mangle]
pub unsafe extern "C" fn PFGLRendererDestroy(renderer: PFGLRendererRef) {
    drop(Box::from_raw(renderer))
}

#[no_mangle]
pub unsafe extern "C" fn PFGLRendererGetDevice(renderer: PFGLRendererRef) -> PFGLDeviceRef {
    &mut (*renderer).device
}

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalDestFramebufferCreateFullWindow(window_size: *const PFVector2I)
                                                                -> PFMetalDestFramebufferRef {
    Box::into_raw(Box::new(DestFramebuffer::full_window((*window_size).to_rust())))
}

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalDestFramebufferDestroy(dest_framebuffer:
                                                       PFMetalDestFramebufferRef) {
    drop(Box::from_raw(dest_framebuffer))
}

/// This function takes ownership of and automatically takes responsibility for destroying `device`
/// and `dest_framebuffer`. However, it does not take ownership of `resources`; therefore, if you
/// created the resource loader, you must destroy it yourself to avoid a memory leak.
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalRendererCreate(device: PFMetalDeviceRef,
                                               resources: PFResourceLoaderRef,
                                               dest_framebuffer: PFMetalDestFramebufferRef,
                                               options: *const PFRendererOptions)
                                               -> PFMetalRendererRef {
    Box::into_raw(Box::new(Renderer::new(*Box::from_raw(device),
                                         &*((*resources).0),
                                         *Box::from_raw(dest_framebuffer),
                                         (*options).to_rust())))
}

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalRendererDestroy(renderer: PFMetalRendererRef) {
    drop(Box::from_raw(renderer))
}

/// Returns a reference to the Metal device in the renderer.
///
/// This reference remains valid as long as the device is alive.
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalRendererGetDevice(renderer: PFMetalRendererRef)
                                                  -> PFMetalDeviceRef {
    &mut (*renderer).device
}

/// This function does not take ownership of `renderer` or `build_options`. Therefore, if you
/// created the renderer and/or options, you must destroy them yourself to avoid a leak.
#[no_mangle]
pub unsafe extern "C" fn PFSceneProxyBuildAndRenderGL(scene_proxy: PFSceneProxyRef,
                                                      renderer: PFGLRendererRef,
                                                      build_options: PFBuildOptionsRef) {
    (*scene_proxy).build_and_render(&mut *renderer, (*build_options).clone())
}

/// This function does not take ownership of `renderer` or `build_options`. Therefore, if you
/// created the renderer and/or options, you must destroy them yourself to avoid a leak.
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFSceneProxyBuildAndRenderMetal(scene_proxy: PFSceneProxyRef,
                                                         renderer: PFMetalRendererRef,
                                                         build_options: PFBuildOptionsRef) {
    (*scene_proxy).build_and_render(&mut *renderer, (*build_options).clone())
}

// `metal`

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalDeviceCreate(layer: *mut CAMetalLayer)
                                             -> PFMetalDeviceRef {
    Box::into_raw(Box::new(MetalDevice::new(CoreAnimationLayerRef::from_ptr(layer))))
}

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalDeviceDestroy(device: PFMetalDeviceRef) {
    drop(Box::from_raw(device))
}

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
#[no_mangle]
pub unsafe extern "C" fn PFMetalDevicePresentDrawable(device: PFMetalDeviceRef) {
    (*device).present_drawable()
}

// `renderer`

#[no_mangle]
pub unsafe extern "C" fn PFRenderTransformCreate2D(transform: *const PFTransform2F)
                                                   -> PFRenderTransformRef {
    Box::into_raw(Box::new(RenderTransform::Transform2D((*transform).to_rust())))
}

#[no_mangle]
pub unsafe extern "C" fn PFRenderTransformCreatePerspective(perspective: *const PFPerspective)
                                                            -> PFRenderTransformRef {
    Box::into_raw(Box::new(RenderTransform::Perspective((*perspective).to_rust())))
}

#[no_mangle]
pub unsafe extern "C" fn PFRenderTransformDestroy(transform: PFRenderTransformRef) {
    drop(Box::from_raw(transform))
}

#[no_mangle]
pub unsafe extern "C" fn PFBuildOptionsCreate() -> PFBuildOptionsRef {
    Box::into_raw(Box::new(BuildOptions::default()))
}

#[no_mangle]
pub unsafe extern "C" fn PFBuildOptionsDestroy(options: PFBuildOptionsRef) {
    drop(Box::from_raw(options))
}

/// Consumes the transform.
#[no_mangle]
pub unsafe extern "C" fn PFBuildOptionsSetTransform(options: PFBuildOptionsRef,
                                                    transform: PFRenderTransformRef) {
    (*options).transform = *Box::from_raw(transform)
}

#[no_mangle]
pub unsafe extern "C" fn PFBuildOptionsSetDilation(options: PFBuildOptionsRef,
                                                   dilation: *const PFVector2F) {
    (*options).dilation = (*dilation).to_rust()
}

#[no_mangle]
pub unsafe extern "C" fn PFBuildOptionsSetSubpixelAAEnabled(options: PFBuildOptionsRef,
                                                            subpixel_aa_enabled: bool) {
    (*options).subpixel_aa_enabled = subpixel_aa_enabled
}

#[no_mangle]
pub unsafe extern "C" fn PFSceneDestroy(scene: PFSceneRef) {
    drop(Box::from_raw(scene))
}

#[no_mangle]
pub unsafe extern "C" fn PFSceneProxyCreateFromSceneAndRayonExecutor(scene: PFSceneRef)
                                                                     -> PFSceneProxyRef {
    Box::into_raw(Box::new(SceneProxy::from_scene(*Box::from_raw(scene), RayonExecutor)))
}

#[no_mangle]
pub unsafe extern "C" fn PFSceneProxyDestroy(scene_proxy: PFSceneProxyRef) {
    drop(Box::from_raw(scene_proxy))
}

// Helpers for `canvas`

unsafe fn to_rust_string(ptr: &*const c_char, mut len: usize) -> &str {
    if len == 0 {
        len = libc::strlen(*ptr);
    }
    str::from_utf8(slice::from_raw_parts(*ptr as *const u8, len)).unwrap()
}

trait TextMetricsExt {
    fn to_c(&self) -> PFTextMetrics;
}

impl TextMetricsExt for TextMetrics {
    fn to_c(&self) -> PFTextMetrics {
        PFTextMetrics { width: self.width }
    }
}

// Helpers for `content`

impl PFColorF {
    #[inline]
    pub fn to_rust(&self) -> ColorF {
        ColorF(F32x4::new(self.r, self.g, self.b, self.a))
    }
}

impl PFColorU {
    #[inline]
    pub fn to_rust(&self) -> ColorU {
        ColorU { r: self.r, g: self.g, b: self.b, a: self.a }
    }
}

// Helpers for `geometry`

impl PFRectF {
    #[inline]
    pub fn to_rust(&self) -> RectF {
        RectF::from_points(self.origin.to_rust(), self.lower_right.to_rust())
    }
}

impl PFRectI {
    #[inline]
    pub fn to_rust(&self) -> RectI {
        RectI::from_points(self.origin.to_rust(), self.lower_right.to_rust())
    }
}

impl PFVector2F {
    #[inline]
    pub fn to_rust(&self) -> Vector2F {
        Vector2F::new(self.x, self.y)
    }
}

impl PFVector2I {
    #[inline]
    pub fn to_rust(&self) -> Vector2I {
        Vector2I::new(self.x, self.y)
    }
}

impl PFMatrix2x2F {
    #[inline]
    pub fn to_rust(&self) -> Matrix2x2F {
        Matrix2x2F::row_major(self.m00, self.m01, self.m10, self.m11)
    }
}

impl PFTransform2F {
    #[inline]
    pub fn to_rust(&self) -> Transform2F {
        Transform2F { matrix: self.matrix.to_rust(), vector: self.vector.to_rust() }
    }
}

impl PFTransform4F {
    #[inline]
    pub fn to_rust(&self) -> Transform4F {
        Transform4F::row_major(self.m00, self.m01, self.m02, self.m03,
                                self.m10, self.m11, self.m12, self.m13,
                                self.m20, self.m21, self.m22, self.m23,
                                self.m30, self.m31, self.m32, self.m33)
    }
}

impl PFPerspective {
    #[inline]
    pub fn to_rust(&self) -> Perspective {
        Perspective {
            transform: self.transform.to_rust(),
            window_size: self.window_size.to_rust(),
        }
    }
}

// Helpers for `renderer`

impl PFRendererOptions {
    pub fn to_rust(&self) -> RendererOptions {
        let has_background_color = self.flags & PF_RENDERER_OPTIONS_FLAGS_HAS_BACKGROUND_COLOR;
        RendererOptions {
            background_color: if has_background_color != 0 {
                Some(self.background_color.to_rust())
            } else {
                None
            },
        }
    }
}
