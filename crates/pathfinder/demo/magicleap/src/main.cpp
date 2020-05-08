// pathfinder/demo/magicleap/src/main.cpp
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// The immersive mode pathfinder magicleap demo

#include <stdio.h>
#include <stdlib.h>

#include <chrono>
#include <cmath>

#include <unistd.h>
#include <sys/syscall.h>

#ifndef EGL_EGLEXT_PROTOTYPES
#define EGL_EGLEXT_PROTOTYPES
#endif

#include <EGL/egl.h>
#include <EGL/eglext.h>

#ifndef GL_GLEXT_PROTOTYPES
#define GL_GLEXT_PROTOTYPES
#endif

#include <GLES3/gl3.h>
#include <GLES3/gl3ext.h>

#include <ml_graphics.h>
#include <ml_head_tracking.h>
#include <ml_perception.h>
#include <ml_fileinfo.h>
#include <ml_lifecycle.h>
#include <ml_logging.h>
#include <ml_privileges.h>

// Entry points to the Rust code
extern "C" void* magicleap_pathfinder_demo_init(EGLDisplay egl_display, EGLContext egl_context);
extern "C" void magicleap_pathfinder_demo_load(void* app, const char* file_name);
extern "C" void magicleap_pathfinder_demo_run(void* app);

// Initialization of the scene thread
extern "C" void init_scene_thread(uint64_t id) {
  // https://forum.magicleap.com/hc/en-us/community/posts/360043120832-How-many-CPUs-does-an-immersive-app-have-access-to-?page=1#community_comment_360005035691
  // We pin scene thread 0 to the Denver core.
  if (id < 3) {
    uint32_t DenverCoreAffinityMask = 1 << (2 + id); // Denver core is CPU2, A57s are CPU3 and 4.
    pid_t ThreadId = gettid();
    syscall(__NR_sched_setaffinity, ThreadId, sizeof(DenverCoreAffinityMask), &DenverCoreAffinityMask);
  }
}

// Constants
const char application_name[] = "com.mozilla.pathfinder.demo";

// Structures
struct graphics_context_t {

  EGLDisplay egl_display;
  EGLContext egl_context;

  GLuint framebuffer_id;
  GLuint vertex_shader_id;
  GLuint fragment_shader_id;
  GLuint program_id;

  graphics_context_t();
  ~graphics_context_t();

  void makeCurrent();
  void swapBuffers();
  void unmakeCurrent();
};

graphics_context_t::graphics_context_t() {
  egl_display = eglGetDisplay(EGL_DEFAULT_DISPLAY);

  EGLint major = 4;
  EGLint minor = 0;
  eglInitialize(egl_display, &major, &minor);
  eglBindAPI(EGL_OPENGL_API);

  EGLint config_attribs[] = {
    EGL_RED_SIZE, 8,
    EGL_GREEN_SIZE, 8,
    EGL_BLUE_SIZE, 8,
    EGL_ALPHA_SIZE, 0,
    EGL_DEPTH_SIZE, 24,
    EGL_STENCIL_SIZE, 8,
    EGL_NONE
  };
  EGLConfig egl_config = nullptr;
  EGLint config_size = 0;
  eglChooseConfig(egl_display, config_attribs, &egl_config, 1, &config_size);

  EGLint context_attribs[] = {
    EGL_CONTEXT_MAJOR_VERSION_KHR, 3,
    EGL_CONTEXT_MINOR_VERSION_KHR, 0,
    EGL_NONE
  };
  egl_context = eglCreateContext(egl_display, egl_config, EGL_NO_CONTEXT, context_attribs);
}

void graphics_context_t::makeCurrent() {
  eglMakeCurrent(egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, egl_context);
}

void graphics_context_t::unmakeCurrent() {
  eglMakeCurrent(NULL, EGL_NO_SURFACE, EGL_NO_SURFACE, NULL);
}

void graphics_context_t::swapBuffers() {
  // buffer swapping is implicit on device (MLGraphicsEndFrame)
}

graphics_context_t::~graphics_context_t() {
  eglDestroyContext(egl_display, egl_context);
  eglTerminate(egl_display);
}

// Callbacks
static void onStop(void* app_handle)
{
  ML_LOG(Info, "%s: On stop called.", application_name);
}

static void onPause(void* app_handle)
{
  ML_LOG(Info, "%s: On pause called.", application_name);
}

static void onResume(void* app_handle)
{
  ML_LOG(Info, "%s: On resume called.", application_name);
}

static void onNewInitArg(void* app_handle)
{
  ML_LOG(Info, "%s: On new init arg called.", application_name);

  // Get the file argument if there is one
  MLLifecycleInitArgList* arg_list = nullptr;
  const MLLifecycleInitArg* arg = nullptr;
  const MLFileInfo* file_info = nullptr;
  const char* file_name = nullptr;
  int64_t arg_list_len = 0;
  int64_t file_list_len = 0;
    
  if (MLResult_Ok != MLLifecycleGetInitArgList(&arg_list)) {
    ML_LOG(Error, "%s: Failed to get init args.", application_name);
    return;
  }

  if (MLResult_Ok != MLLifecycleGetInitArgListLength(arg_list, &arg_list_len)) {
    ML_LOG(Error, "%s: Failed to get init arg length.", application_name);
    return;
  }

  if (!arg_list_len) {
      return;
  }

  if (MLResult_Ok != MLLifecycleGetInitArgByIndex(arg_list, 0, &arg)) {
    ML_LOG(Error, "%s: Failed to get init arg.", application_name);
  }

  if (MLResult_Ok != MLLifecycleGetFileInfoListLength(arg, &file_list_len)) {
    ML_LOG(Error, "%s: Failed to get file list length.", application_name);
    return;
  }

  if (!file_list_len) {
    return;
  }
  
  if (MLResult_Ok != MLLifecycleGetFileInfoByIndex(arg, 0, &file_info)) {
    ML_LOG(Error, "%s: Failed to get file info.", application_name);
    return;
  }

  if (MLResult_Ok != MLFileInfoGetFileName(file_info, &file_name)) {
    ML_LOG(Error, "%s: Failed to get file name.", application_name);
    return;
  }

  if (!file_name) {
    ML_LOG(Error, "%s: File name is null.", application_name);
    return;
  }

  // Tell pathfinder to load the file
  if (!app_handle) {
    ML_LOG(Error, "%s: Init arg set before app is initialized.", application_name);
    return;
  }
  void* app = *((void**)app_handle);
  if (!app) {
    ML_LOG(Error, "%s: Init arg set before app is initialized.", application_name);
    return;
  }

  ML_LOG(Info, "%s: Loading %s.", application_name, file_name);
  magicleap_pathfinder_demo_load(app, file_name);
  MLLifecycleFreeInitArgList(&arg_list);
}

extern "C" void logMessage(MLLogLevel lvl, char* msg) {
  if (MLLoggingLogLevelIsEnabled(lvl)) {
    MLLoggingLog(lvl, ML_DEFAULT_LOG_TAG, msg);
  }
}

int main() {
  // set up host-specific graphics surface
  graphics_context_t graphics_context;

  // the app will go here once it's initialized
  void* app = nullptr;

  // let system know our app has started
  MLLifecycleCallbacks lifecycle_callbacks = {};
  lifecycle_callbacks.on_stop = onStop;
  lifecycle_callbacks.on_pause = onPause;
  lifecycle_callbacks.on_resume = onResume;
  lifecycle_callbacks.on_new_initarg = onNewInitArg;

  if (MLResult_Ok != MLLifecycleInit(&lifecycle_callbacks, &app)) {
    ML_LOG(Error, "%s: Failed to initialize lifecycle.", application_name);
    return -1;
  }

  // Check privileges
  if (MLResult_Ok != MLPrivilegesStartup()) {
    ML_LOG(Error, "%s: Failed to initialize privileges.", application_name);
    return -1;
  }
  if (MLPrivilegesRequestPrivilege(MLPrivilegeID_WorldReconstruction) != MLPrivilegesResult_Granted) {
    ML_LOG(Error, "Privilege %d denied.", MLPrivilegeID_WorldReconstruction);
    return -1;
  }
  if (MLPrivilegesRequestPrivilege(MLPrivilegeID_LowLatencyLightwear) != MLPrivilegesResult_Granted) {
    ML_LOG(Error, "Privilege %d denied.", MLPrivilegeID_LowLatencyLightwear);
    return -1;
  }
  
  // initialize perception system
  MLPerceptionSettings perception_settings;
  if (MLResult_Ok != MLPerceptionInitSettings(&perception_settings)) {
    ML_LOG(Error, "%s: Failed to initialize perception.", application_name);
  }

  if (MLResult_Ok != MLPerceptionStartup(&perception_settings)) {
    ML_LOG(Error, "%s: Failed to startup perception.", application_name);
    return -1;
  }

  // Initialize pathfinder
  ML_LOG(Info, "%s: Initializing demo.", application_name);
  app = magicleap_pathfinder_demo_init(graphics_context.egl_display, graphics_context.egl_context);
  if (!app) {
    ML_LOG(Error, "%s: Failed to initialize demo.", application_name);
  }

  // Get the initial argument if there is one.
  onNewInitArg(&app);

  // Run the demo!
  ML_LOG(Info, "%s: Begin demo.", application_name);
  magicleap_pathfinder_demo_run(app);
  ML_LOG(Info, "%s: End demo.", application_name);

  // Shut down
  MLPerceptionShutdown();

  return 0;
}
