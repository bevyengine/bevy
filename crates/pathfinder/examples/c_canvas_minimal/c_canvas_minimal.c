// pathfinder/examples/c_canvas_minimal/c_canvas_minimal.c
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#include <SDL2/SDL.h>
#include <SDL2/SDL_opengl.h>
#include <pathfinder/pathfinder.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

static const void *LoadGLFunction(const char *name, void *userdata);
static void SDLFailed(const char *msg);

int main(int argc, const char **argv) {
    // Set up SDL2.
    if (SDL_Init(SDL_INIT_EVENTS | SDL_INIT_VIDEO) != 0)
        SDLFailed("Failed to initialize SDL");

    // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
    if (SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3) != 0)
        SDLFailed("Failed to set GL major version");
    if (SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 2) != 0)
        SDLFailed("Failed to set GL minor version");
    if (SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE) != 0)
        SDLFailed("Failed to set GL profile");
    if (SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1) != 0)
        SDLFailed("Failed to make GL context double-buffered");

    // Open a window.
    SDL_Window *window = SDL_CreateWindow("Minimal canvas example (C API)",
                                          SDL_WINDOWPOS_CENTERED,
                                          SDL_WINDOWPOS_CENTERED,
                                          640,
                                          480,
                                          SDL_WINDOW_OPENGL);
    if (window == NULL)
        SDLFailed("Failed to create SDL window");

    // Create the GL context, and make it current.
    SDL_GLContext gl_context = SDL_GL_CreateContext(window);
    if (gl_context == NULL)
        SDLFailed("Failed to create GL context");

    // Put the window on screen.
    SDL_ShowWindow(window);

    // Create a Pathfinder renderer.
    PFGLLoadWith(LoadGLFunction, NULL);
    PFGLDestFramebufferRef dest_framebuffer =
        PFGLDestFramebufferCreateFullWindow(&(PFVector2I){640, 480});
    PFGLRendererRef renderer = PFGLRendererCreate(PFGLDeviceCreate(PF_GL_VERSION_GL3, 0),
                                                  PFFilesystemResourceLoaderLocate(),
                                                  dest_framebuffer,
                                                  &(PFRendererOptions){
        (PFColorF){1.0, 1.0, 1.0, 1.0}, PF_RENDERER_OPTIONS_FLAGS_HAS_BACKGROUND_COLOR
    });

    // Make a canvas. We're going to draw a house.
    PFCanvasRef canvas = PFCanvasCreate(PFCanvasFontContextCreateWithSystemSource(),
                                        &(PFVector2F){640.0f, 480.0f});

    // Set line width.
    PFCanvasSetLineWidth(canvas, 10.0f);

    // Draw walls.
    PFCanvasStrokeRect(canvas, &(PFRectF){{75.0f, 140.0f}, {225.0f, 250.0f}});

    // Draw door.
    PFCanvasFillRect(canvas, &(PFRectF){{130.0f, 190.0f}, {170.0f, 250.0f}});

    // Draw roof.
    PFPathRef path = PFPathCreate();
    PFPathMoveTo(path, &(PFVector2F){50.0, 140.0});
    PFPathLineTo(path, &(PFVector2F){150.0, 60.0});
    PFPathLineTo(path, &(PFVector2F){250.0, 140.0});
    PFPathClosePath(path);
    PFCanvasStrokePath(canvas, path);

    // Render the canvas to screen.
    PFSceneRef scene = PFCanvasCreateScene(canvas);
    PFSceneProxyRef scene_proxy = PFSceneProxyCreateFromSceneAndRayonExecutor(scene);
    PFSceneProxyBuildAndRenderGL(scene_proxy, renderer, PFBuildOptionsCreate());
    SDL_GL_SwapWindow(window);

    // Wait for a keypress.
    while (true) {
        SDL_Event event;
        if (SDL_WaitEvent(&event) == 0)
            SDLFailed("Failed to get SDL event");
        if (event.type == SDL_QUIT ||
            (event.type == SDL_KEYDOWN && event.key.keysym.sym == SDLK_ESCAPE)) {
            break;
        }
    }

    // Finish up.
    SDL_Quit();
    return 0;
}

static const void *LoadGLFunction(const char *name, void *userdata) {
    return SDL_GL_GetProcAddress(name);
}

static void SDLFailed(const char *msg) {
    fprintf(stderr, "%s: %s\n", msg, SDL_GetError());
    exit(EXIT_FAILURE);
}
