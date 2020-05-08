//
//  PathfinderView.m
//  Pathfinder Example
//
//  Created by Patrick Walton on 6/21/19.
//  Copyright © 2019 The Pathfinder Project Developers. All rights reserved.
//

#import <QuartzCore/QuartzCore.h>
#import "PathfinderView.h"
#import <Metal/Metal.h>
#include <math.h>

static CVReturn outputCallback(CVDisplayLinkRef displayLink,
                               const CVTimeStamp *now,
                               const CVTimeStamp *outputTime,
                               CVOptionFlags flagsIn,
                               CVOptionFlags *flagsOut,
                               void *userData) {
    [(__bridge PathfinderView *)userData _render];
    return kCVReturnSuccess;
}

static CATransform3D createPerspectiveMatrix(CGFloat fovY,
                                             CGFloat aspect,
                                             CGFloat zNear,
                                             CGFloat zFar) {
    CGFloat f = tan(1.0 / (fovY * 0.5));
    CGFloat zDenom = 1.0 / (zNear - zFar);

    CATransform3D transform = CATransform3DIdentity;
    transform.m11 = f / aspect;
    transform.m22 = f;
    transform.m33 = (zFar + zNear) * zDenom;
    transform.m34 = -1.0;
    transform.m43 = 2.0 * zFar * zNear * zDenom;
    return transform;
}

static PFTransform3DF pfTransformFromCATransform(const CATransform3D *transform) {
    // Core Animation matrices are in column-major order, while Pathfinder matrices are in
    // row-major order (at least in the latter's C API). So transpose here.
    PFTransform3DF pfTransform;
    pfTransform.m00 = (float)transform->m11;
    pfTransform.m01 = (float)transform->m21;
    pfTransform.m02 = (float)transform->m31;
    pfTransform.m03 = (float)transform->m41;
    pfTransform.m10 = (float)transform->m12;
    pfTransform.m11 = (float)transform->m22;
    pfTransform.m12 = (float)transform->m32;
    pfTransform.m13 = (float)transform->m42;
    pfTransform.m20 = (float)transform->m13;
    pfTransform.m21 = (float)transform->m23;
    pfTransform.m22 = (float)transform->m33;
    pfTransform.m23 = (float)transform->m43;
    pfTransform.m30 = (float)transform->m14;
    pfTransform.m31 = (float)transform->m24;
    pfTransform.m32 = (float)transform->m34;
    pfTransform.m33 = (float)transform->m44;
    return pfTransform;
}

@implementation PathfinderView

#define FONT_SIZE   256.0f

- (void)_render {
    [mRenderLock lock];

    CGSize size = mLayerSize;

    PFCanvasRef canvas = PFCanvasCreate(mFontContext, &(PFVector2F){size.width, size.height});
    PFFillStyleRef fillStyle =
    PFFillStyleCreateColor(&(PFColorU){0, 0, 0, 255});
    PFCanvasSetFillStyle(canvas, fillStyle);
    PFCanvasSetFontSize(canvas, FONT_SIZE);
    PFCanvasSetTextAlign(canvas, PF_TEXT_ALIGN_CENTER);
    PFVector2F textOrigin;
    textOrigin.x = 0.0;
    textOrigin.y = FONT_SIZE * 0.25;
    PFCanvasFillText(canvas, "Pathfinder", 0, &textOrigin);
    PFCanvasFillRect(canvas, &(const PFRectF){0.0, 0.0, 1.0, 1.0});
    PFFillStyleDestroy(fillStyle);

    PFSceneRef scene = PFCanvasCreateScene(canvas);
    PFSceneProxyRef sceneProxy = PFSceneProxyCreateFromSceneAndRayonExecutor(scene);

    int32_t frame = mFrameNumber;
    int32_t nT = frame % 240;
    if (nT > 120)
        nT = 240 - nT;

    CATransform3D transform =
        CATransform3DMakeTranslation(0.0, 0.0, -8.0 + (CGFloat)nT / 120.0 * 8.0);
    transform = CATransform3DRotate(transform,
                                    frame / 120.0 * M_PI * 2.0,
                                    0.0,
                                    1.0,
                                    0.0);
    transform = CATransform3DScale(transform, -2.0 / size.width, 2.0 / size.height, 1.0);
    CGFloat aspect = size.width / size.height;
    transform = CATransform3DConcat(transform,
                                    createPerspectiveMatrix(M_PI * 0.25, aspect, 0.01, 10.0));
    PFPerspective pfPerspective;
    pfPerspective.transform = pfTransformFromCATransform(&transform);
    pfPerspective.window_size.x = size.width;
    pfPerspective.window_size.y = size.height;

    PFBuildOptionsRef buildOptions = PFBuildOptionsCreate();
    PFRenderTransformRef renderTransform = PFRenderTransformCreatePerspective(&pfPerspective);
    PFBuildOptionsSetTransform(buildOptions, renderTransform);
    PFSceneProxyBuildAndRenderMetal(sceneProxy, mRenderer, buildOptions);

    PFMetalDevicePresentDrawable(PFMetalRendererGetDevice(mRenderer));

    mFrameNumber++;

    [mRenderLock unlock];
}

- (void)_checkCVResult:(CVReturn)result {
    if (result != kCVReturnSuccess) {
        @throw [NSException exceptionWithName:@"CoreVideoCallFailed"
                                       reason:@"Core Video call failed"
                                     userInfo:nil];
    }
}

- (void)_initializeIfNecessary:(CAMetalLayer *)layer {
    if (mDevice != nil)
        return;

    mFrameNumber = 0;

    mDevice = MTLCreateSystemDefaultDevice();
    [layer setDevice:mDevice];
    [layer setContentsScale:[[self window] backingScaleFactor]];

    mRenderLock = [[NSLock alloc] init];
    mLayerSize = [self convertSizeToBacking:[layer bounds].size];

    PFMetalDeviceRef device = PFMetalDeviceCreate(layer);
    PFResourceLoaderRef resourceLoader = PFFilesystemResourceLoaderLocate();
    PFMetalDestFramebufferRef destFramebuffer =
    PFMetalDestFramebufferCreateFullWindow(&(PFVector2I){mLayerSize.width, mLayerSize.height});

    PFRendererOptions rendererOptions;
    rendererOptions.background_color = (PFColorF){1.0, 1.0, 1.0, 1.0};
    rendererOptions.flags = PF_RENDERER_OPTIONS_FLAGS_HAS_BACKGROUND_COLOR;
    mRenderer = PFMetalRendererCreate(device,
                                      resourceLoader,
                                      destFramebuffer,
                                      &rendererOptions);

    mFontContext = PFCanvasFontContextCreateWithSystemSource();

    mBuildOptions = PFBuildOptionsCreate();

    [self _checkCVResult:CVDisplayLinkCreateWithActiveCGDisplays(&mDisplayLink)];
    [self _checkCVResult:CVDisplayLinkSetOutputCallback(mDisplayLink,
                                                        outputCallback,
                                                        (__bridge void *_Nullable)(self))];
    [self _checkCVResult:CVDisplayLinkStart(mDisplayLink)];
}

- (CALayer *)makeBackingLayer {
    return [[CAMetalLayer alloc] init];
}

- (BOOL)wantsLayer {
    return YES;
}

- (BOOL)wantsUpdateLayer {
    return YES;
}

- (NSViewLayerContentsRedrawPolicy)layerContentsRedrawPolicy {
    return NSViewLayerContentsRedrawOnSetNeedsDisplay;
}

- (void)drawRect:(NSRect)dirtyRect {
    [self _initializeIfNecessary:(CAMetalLayer *)[self layer]];
}

- (void)displayLayer:(CALayer *)layer {
    [self _initializeIfNecessary:(CAMetalLayer *)layer];
}

- (void)awakeFromNib {
    [self _initializeIfNecessary:(CAMetalLayer *)[self layer]];
}

@end
