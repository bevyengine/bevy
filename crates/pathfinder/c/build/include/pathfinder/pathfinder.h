/* Generated code. Do not edit; instead run `cargo build` in `pathfinder_c`. */

#ifndef PF_PATHFINDER_H
#define PF_PATHFINDER_H

#ifdef __APPLE__
#include <QuartzCore/QuartzCore.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif


/* Generated with cbindgen:0.13.2 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define PF_ARC_DIRECTION_CCW 1

#define PF_ARC_DIRECTION_CW 0

#define PF_GL_VERSION_GL3 0

#define PF_GL_VERSION_GLES3 1

#define PF_LINE_CAP_BUTT 0

#define PF_LINE_CAP_ROUND 2

#define PF_LINE_CAP_SQUARE 1

#define PF_LINE_JOIN_BEVEL 1

#define PF_LINE_JOIN_MITER 0

#define PF_LINE_JOIN_ROUND 2

#define PF_RENDERER_OPTIONS_FLAGS_HAS_BACKGROUND_COLOR 1

#define PF_TEXT_ALIGN_CENTER 1

#define PF_TEXT_ALIGN_LEFT 0

#define PF_TEXT_ALIGN_RIGHT 2

/**
 * Options that influence scene building.
 */
typedef struct PFBuildOptionsPrivate PFBuildOptionsPrivate;

typedef struct PFCanvasFontContextPrivate PFCanvasFontContextPrivate;

typedef struct PFCanvasFontContextPrivate PFCanvasFontContextPrivate;

typedef struct PFCanvasRenderingContext2DPrivate PFCanvasRenderingContext2DPrivate;

typedef struct PFDestFramebufferGLDevicePrivate PFDestFramebufferGLDevicePrivate;

typedef struct PFDestFramebufferMetalDevicePrivate PFDestFramebufferMetalDevicePrivate;

typedef struct PFFillStylePrivate PFFillStylePrivate;

typedef struct PFGLDevicePrivate PFGLDevicePrivate;

/**
 * Encapsulates the information needed to locate and open a font.
 *
 * This is either the path to the font or the raw in-memory font data.
 *
 * To open the font referenced by a handle, use a loader.
 */
typedef struct FKHandlePrivate FKHandlePrivate;

typedef struct PFMetalDevicePrivate PFMetalDevicePrivate;

typedef struct PFPath2DPrivate PFPath2DPrivate;

typedef struct PFRenderTransformPrivate PFRenderTransformPrivate;

typedef struct PFRendererGLDevicePrivate PFRendererGLDevicePrivate;

typedef struct PFRendererMetalDevicePrivate PFRendererMetalDevicePrivate;

typedef struct PFResourceLoaderWrapperPrivate PFResourceLoaderWrapperPrivate;

typedef struct PFScenePrivate PFScenePrivate;

typedef struct PFSceneProxyPrivate PFSceneProxyPrivate;

typedef PFBuildOptionsPrivate *PFBuildOptionsRef;

typedef struct {
  float x;
  float y;
} PFVector2F;

typedef PFRenderTransformPrivate *PFRenderTransformRef;

typedef PFCanvasRenderingContext2DPrivate *PFCanvasRef;

typedef PFCanvasFontContextPrivate *PFCanvasFontContextRef;

typedef PFScenePrivate *PFSceneRef;

typedef PFPath2DPrivate *PFPathRef;

typedef struct {
  PFVector2F origin;
  PFVector2F lower_right;
} PFRectF;

typedef FKHandlePrivate *FKHandleRef;

typedef struct {
  float width;
} PFTextMetrics;

typedef PFFillStylePrivate *PFFillStyleRef;

typedef uint8_t PFLineCap;

typedef uint8_t PFLineJoin;

typedef uint8_t PFTextAlign;

/**
 * Row-major order.
 */
typedef struct {
  float m00;
  float m01;
  float m10;
  float m11;
} PFMatrix2x2F;

/**
 * Row-major order.
 */
typedef struct {
  PFMatrix2x2F matrix;
  PFVector2F vector;
} PFTransform2F;

typedef PFResourceLoaderWrapperPrivate *PFResourceLoaderRef;

typedef struct {
  uint8_t r;
  uint8_t g;
  uint8_t b;
  uint8_t a;
} PFColorU;

typedef PFDestFramebufferGLDevicePrivate *PFGLDestFramebufferRef;

typedef struct {
  int32_t x;
  int32_t y;
} PFVector2I;

typedef PFGLDevicePrivate *PFGLDeviceRef;

typedef uint8_t PFGLVersion;

typedef const void *(*PFGLFunctionLoader)(const char *name, void *userdata);

typedef PFRendererGLDevicePrivate *PFGLRendererRef;

typedef struct {
  float r;
  float g;
  float b;
  float a;
} PFColorF;

typedef uint8_t PFRendererOptionsFlags;

typedef struct {
  PFColorF background_color;
  PFRendererOptionsFlags flags;
} PFRendererOptions;

typedef PFDestFramebufferMetalDevicePrivate *PFMetalDestFramebufferRef;

typedef PFMetalDevicePrivate *PFMetalDeviceRef;

typedef PFRendererMetalDevicePrivate *PFMetalRendererRef;

typedef uint8_t PFArcDirection;

/**
 * Row-major order.
 */
typedef struct {
  float m00;
  float m01;
  float m02;
  float m03;
  float m10;
  float m11;
  float m12;
  float m13;
  float m20;
  float m21;
  float m22;
  float m23;
  float m30;
  float m31;
  float m32;
  float m33;
} PFTransform4F;

typedef struct {
  PFTransform4F transform;
  PFVector2I window_size;
} PFPerspective;

typedef PFSceneProxyPrivate *PFSceneProxyRef;

PFBuildOptionsRef PFBuildOptionsCreate(void);

void PFBuildOptionsDestroy(PFBuildOptionsRef options);

void PFBuildOptionsSetDilation(PFBuildOptionsRef options, const PFVector2F *dilation);

void PFBuildOptionsSetSubpixelAAEnabled(PFBuildOptionsRef options, bool subpixel_aa_enabled);

/**
 * Consumes the transform.
 */
void PFBuildOptionsSetTransform(PFBuildOptionsRef options, PFRenderTransformRef transform);

/**
 * This function internally adds a reference to the font context. Therefore, if you created the
 * font context, you must release it yourself to avoid a leak.
 */
PFCanvasRef PFCanvasCreate(PFCanvasFontContextRef font_context, const PFVector2F *size);

/**
 * This function takes ownership of the supplied canvas and will automatically destroy it when
 * the scene is destroyed.
 */
PFSceneRef PFCanvasCreateScene(PFCanvasRef canvas);

void PFCanvasDestroy(PFCanvasRef canvas);

/**
 * This function automatically destroys the path. If you wish to use the path again, clone it
 * first.
 */
void PFCanvasFillPath(PFCanvasRef canvas, PFPathRef path);

void PFCanvasFillRect(PFCanvasRef canvas, const PFRectF *rect);

void PFCanvasFillText(PFCanvasRef canvas,
                      const char *string,
                      uintptr_t string_len,
                      const PFVector2F *position);

PFCanvasFontContextRef PFCanvasFontContextAddRef(PFCanvasFontContextRef font_context);

PFCanvasFontContextRef PFCanvasFontContextCreateWithFonts(const FKHandleRef *fonts,
                                                          uintptr_t font_count);

PFCanvasFontContextRef PFCanvasFontContextCreateWithSystemSource(void);

void PFCanvasFontContextRelease(PFCanvasFontContextRef font_context);

void PFCanvasMeasureText(PFCanvasRef canvas,
                         const char *string,
                         uintptr_t string_len,
                         PFTextMetrics *out_text_metrics);

void PFCanvasResetTransform(PFCanvasRef canvas);

void PFCanvasRestore(PFCanvasRef canvas);

void PFCanvasSave(PFCanvasRef canvas);

void PFCanvasSetFillStyle(PFCanvasRef canvas, PFFillStyleRef fill_style);

void PFCanvasSetFontByPostScriptName(PFCanvasRef canvas,
                                     const char *postscript_name,
                                     uintptr_t postscript_name_len);

void PFCanvasSetFontSize(PFCanvasRef canvas, float new_font_size);

void PFCanvasSetLineCap(PFCanvasRef canvas, PFLineCap new_line_cap);

void PFCanvasSetLineDash(PFCanvasRef canvas,
                         const float *new_line_dashes,
                         uintptr_t new_line_dash_count);

void PFCanvasSetLineDashOffset(PFCanvasRef canvas, float new_offset);

void PFCanvasSetLineJoin(PFCanvasRef canvas, PFLineJoin new_line_join);

void PFCanvasSetLineWidth(PFCanvasRef canvas, float new_line_width);

void PFCanvasSetMiterLimit(PFCanvasRef canvas, float new_miter_limit);

void PFCanvasSetStrokeStyle(PFCanvasRef canvas, PFFillStyleRef stroke_style);

void PFCanvasSetTextAlign(PFCanvasRef canvas, PFTextAlign new_text_align);

void PFCanvasSetTransform(PFCanvasRef canvas, const PFTransform2F *transform);

/**
 * This function automatically destroys the path. If you wish to use the path again, clone it
 * first.
 */
void PFCanvasStrokePath(PFCanvasRef canvas, PFPathRef path);

void PFCanvasStrokeRect(PFCanvasRef canvas, const PFRectF *rect);

void PFCanvasStrokeText(PFCanvasRef canvas,
                        const char *string,
                        uintptr_t string_len,
                        const PFVector2F *position);

PFResourceLoaderRef PFFilesystemResourceLoaderLocate(void);

PFFillStyleRef PFFillStyleCreateColor(const PFColorU *color);

void PFFillStyleDestroy(PFFillStyleRef fill_style);

PFGLDestFramebufferRef PFGLDestFramebufferCreateFullWindow(const PFVector2I *window_size);

void PFGLDestFramebufferDestroy(PFGLDestFramebufferRef dest_framebuffer);

PFGLDeviceRef PFGLDeviceCreate(PFGLVersion version, uint32_t default_framebuffer);

void PFGLDeviceDestroy(PFGLDeviceRef device);

void PFGLLoadWith(PFGLFunctionLoader loader, void *userdata);

/**
 * This function takes ownership of and automatically takes responsibility for destroying `device`
 * and `dest_framebuffer`. However, it does not take ownership of `resources`; therefore, if you
 * created the resource loader, you must destroy it yourself to avoid a memory leak.
 */
PFGLRendererRef PFGLRendererCreate(PFGLDeviceRef device,
                                   PFResourceLoaderRef resources,
                                   PFGLDestFramebufferRef dest_framebuffer,
                                   const PFRendererOptions *options);

void PFGLRendererDestroy(PFGLRendererRef renderer);

PFGLDeviceRef PFGLRendererGetDevice(PFGLRendererRef renderer);

PFMetalDestFramebufferRef PFMetalDestFramebufferCreateFullWindow(const PFVector2I *window_size);

void PFMetalDestFramebufferDestroy(PFMetalDestFramebufferRef dest_framebuffer);

PFMetalDeviceRef PFMetalDeviceCreate(CAMetalLayer *layer);

void PFMetalDeviceDestroy(PFMetalDeviceRef device);

void PFMetalDevicePresentDrawable(PFMetalDeviceRef device);

/**
 * This function takes ownership of and automatically takes responsibility for destroying `device`
 * and `dest_framebuffer`. However, it does not take ownership of `resources`; therefore, if you
 * created the resource loader, you must destroy it yourself to avoid a memory leak.
 */
PFMetalRendererRef PFMetalRendererCreate(PFMetalDeviceRef device,
                                         PFResourceLoaderRef resources,
                                         PFMetalDestFramebufferRef dest_framebuffer,
                                         const PFRendererOptions *options);

void PFMetalRendererDestroy(PFMetalRendererRef renderer);

/**
 * Returns a reference to the Metal device in the renderer.
 *
 * This reference remains valid as long as the device is alive.
 */
PFMetalDeviceRef PFMetalRendererGetDevice(PFMetalRendererRef renderer);

void PFPathArc(PFPathRef path,
               const PFVector2F *center,
               float radius,
               float start_angle,
               float end_angle,
               PFArcDirection direction);

void PFPathArcTo(PFPathRef path, const PFVector2F *ctrl, const PFVector2F *to, float radius);

void PFPathBezierCurveTo(PFPathRef path,
                         const PFVector2F *ctrl0,
                         const PFVector2F *ctrl1,
                         const PFVector2F *to);

PFPathRef PFPathClone(PFPathRef path);

void PFPathClosePath(PFPathRef path);

PFPathRef PFPathCreate(void);

void PFPathDestroy(PFPathRef path);

void PFPathEllipse(PFPathRef path,
                   const PFVector2F *center,
                   const PFVector2F *axes,
                   float rotation,
                   float start_angle,
                   float end_angle);

void PFPathLineTo(PFPathRef path, const PFVector2F *to);

void PFPathMoveTo(PFPathRef path, const PFVector2F *to);

void PFPathQuadraticCurveTo(PFPathRef path, const PFVector2F *ctrl, const PFVector2F *to);

void PFPathRect(PFPathRef path, const PFRectF *rect);

PFRenderTransformRef PFRenderTransformCreate2D(const PFTransform2F *transform);

PFRenderTransformRef PFRenderTransformCreatePerspective(const PFPerspective *perspective);

void PFRenderTransformDestroy(PFRenderTransformRef transform);

void PFResourceLoaderDestroy(PFResourceLoaderRef loader);

void PFSceneDestroy(PFSceneRef scene);

/**
 * This function does not take ownership of `renderer` or `build_options`. Therefore, if you
 * created the renderer and/or options, you must destroy them yourself to avoid a leak.
 */
void PFSceneProxyBuildAndRenderGL(PFSceneProxyRef scene_proxy,
                                  PFGLRendererRef renderer,
                                  PFBuildOptionsRef build_options);

/**
 * This function does not take ownership of `renderer` or `build_options`. Therefore, if you
 * created the renderer and/or options, you must destroy them yourself to avoid a leak.
 */
void PFSceneProxyBuildAndRenderMetal(PFSceneProxyRef scene_proxy,
                                     PFMetalRendererRef renderer,
                                     PFBuildOptionsRef build_options);

PFSceneProxyRef PFSceneProxyCreateFromSceneAndRayonExecutor(PFSceneRef scene);

void PFSceneProxyDestroy(PFSceneProxyRef scene_proxy);

#ifdef __cplusplus
}
#endif

#endif
