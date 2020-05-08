// pathfinder/demo/magicleap/src/landscape.cpp
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// A launcher for the ML1 pathfinder demo

#include <landscape.h>
#include <lumin/node/RootNode.h>
#include <lumin/node/QuadNode.h>
#include <lumin/resource/PlanarResource.h>
#include <lumin/ui/node/UiPanel.h>
#include <lumin/ui/Cursor.h>
#include <lumin/input/Raycast.h>
#include <lumin/event/ControlPose6DofInputEventData.h>
#include <lumin/event/GestureInputEventData.h>
#include <lumin/event/RayCastEventData.h>
#include <ml_dispatch.h>
#include <ml_logging.h>
#include <scenes.h>
#include <PrismSceneManager.h>

int main(int argc, char **argv)
{
  ML_LOG(Debug, "PathfinderDemo Starting.");
  PathfinderDemo myApp;
  return myApp.run();
}

const char* QUAD_NAMES[NUM_QUADS] = {
  "quad1",
  "quad2",
  "quad3",
  "quad4",
  "quad5",
  "quad6",
};

const char* SVG_NAMES[NUM_QUADS] = {
  "svg/Ghostscript_Tiger.svg",
  "svg/nba-notext.svg",
  "svg/julius-caesar-with-bg.svg",
  "svg/magicleap-quickstart-p03.svg",
  "svg/pathfinder_logo.svg",
  "svg/pathfinder-magicleap-demo.svg",
};

PathfinderDemo::PathfinderDemo() {
  ML_LOG(Debug, "PathfinderDemo Constructor.");

  // Place your constructor implementation here.
}

PathfinderDemo::~PathfinderDemo() {
  ML_LOG(Debug, "PathfinderDemo Destructor.");

  // Place your destructor implementation here.
}

const glm::vec3 PathfinderDemo::getInitialPrismSize() const {
  return glm::vec3(0.4f, 0.4f, 0.4f);
}

void PathfinderDemo::createInitialPrism() {
  prism_ = requestNewPrism(getInitialPrismSize());
  if (!prism_) {
    ML_LOG(Error, "PathfinderDemo Error creating default prism.");
    abort();
  }
  prismSceneManager_ = new PrismSceneManager(prism_);
}

int PathfinderDemo::init() {

  ML_LOG(Debug, "PathfinderDemo Initializing.");

  createInitialPrism();
  lumin::ui::Cursor::SetEnabled(prism_, false);
  spawnInitialScenes();

  // Place your initialization here.
  if (checkPrivilege(lumin::PrivilegeId::kControllerPose) != lumin::PrivilegeResult::kGranted) {
    ML_LOG(Error, "Pathfinder Failed to get controller access");
    abort();
    return 1;
  }

  // Get the root node of the prism
  lumin::RootNode* root_node = prism_->getRootNode();
  if (!root_node) {
    ML_LOG(Error, "Pathfinder Failed to get root node");
    abort();
    return 1;
  }

  for (int i=0; i<NUM_QUADS; i++) {
  // Get the quad
  lumin::QuadNode* quad_node = lumin::QuadNode::CastFrom(prism_->findNode(QUAD_NAMES[i], root_node));
  if (!quad_node) {
    ML_LOG(Error, "Pathfinder Failed to get quad node %d.", i);
    abort();
    return 1;
  }
  quad_nodes_[i] = quad_node->getNodeId();

  // Create the EGL surface for it to draw to
  lumin::ResourceIDType plane_id = prism_->createPlanarEGLResourceId();
  if (!plane_id) {
    ML_LOG(Error, "Pathfinder Failed to create EGL resource");
    abort();
    return 1;
  }
  quad_node->setRenderResource(plane_id);

  renderNode(quad_node->getNodeId());
  }

  return 0;
}

void PathfinderDemo::renderNode(lumin::NodeIDType node_id) {
  if (node_id == lumin::INVALID_NODE_ID) { return; }

  lumin::QuadNode* quad_node = static_cast<lumin::QuadNode*>(prism_->getNode(node_id));
  if (!quad_node) {
    ML_LOG(Error, "Pathfinder Failed to get quad node");
    return;
  }

  lumin::PlanarResource* plane = const_cast<lumin::PlanarResource*>(static_cast<const lumin::PlanarResource*>(quad_node->getRenderResource()));
  if (!plane) {
    ML_LOG(Error, "Pathfinder Failed to get plane");
    return;
  }

  // Get the EGL context, surface and display.
  uint32_t width = plane->getWidth();
  uint32_t height = plane->getHeight();
  EGLContext ctx = plane->getEGLContext();
  EGLSurface surf = plane->getEGLSurface();
  EGLDisplay dpy = eglGetDisplay(EGL_DEFAULT_DISPLAY);

  // Initialize pathfinder
  if (!pathfinder_) {
    ML_LOG(Info, "Pathfinder initializing");
    pathfinder_ = magicleap_pathfinder_init();
    ML_LOG(Info, "Pathfinder initialized");
  }

  // Get the SVG filename
  const char* svg_filename;
  for (int i=0; i<NUM_QUADS; i++) {
    if (quad_nodes_[i] == node_id) {
      svg_filename = SVG_NAMES[i];
      break;
    }
  }
  if (!svg_filename) {
    ML_LOG(Error, "Pathfinder Failed to get SVG filename");
    return;
  }

  // Set the brightness
  float brightness;
  if (node_id == highlighted_node_) {
    brightness = 0.6;
  } else {
    brightness = 0.4;
  }

  // Render the SVG
  MagicLeapPathfinderRenderOptions options = {
    dpy,
    surf,
    { brightness, brightness, brightness, 1.0 },
    { 0, 0, width, height },
    svg_filename,
  };

  eglMakeCurrent(dpy, surf, surf, ctx);
  magicleap_pathfinder_render(pathfinder_, &options);
  eglSwapBuffers(dpy, surf);
}

int PathfinderDemo::deInit() {
  ML_LOG(Debug, "PathfinderDemo Deinitializing.");

  // Place your deinitialization here.
  magicleap_pathfinder_deinit(pathfinder_);
  pathfinder_ = nullptr;

  return 0;
}

void PathfinderDemo::spawnInitialScenes() {

  // Iterate over all the exported scenes
  for (auto& exportedSceneEntry : scenes::externalScenes ) {

    // If this scene was marked to be instanced at app initialization, do it
    const SceneDescriptor &sd = exportedSceneEntry.second;
    if (sd.getInitiallySpawned()) {
      lumin::Node* const spawnedRoot = prismSceneManager_->spawn(sd);
      if (spawnedRoot) {
        if (!prism_->getRootNode()->addChild(spawnedRoot)) {
          ML_LOG(Error, "PathfinderDemo Failed to add spawnedRoot to the prism root node");
          abort();
        }
      }
    }
  }
}

bool PathfinderDemo::updateLoop(float fDelta) {
  // Place your update here.
  if (focus_node_ != highlighted_node_) {
    lumin::NodeIDType old_highlight = highlighted_node_;
    highlighted_node_ = focus_node_;
    renderNode(old_highlight);
    renderNode(highlighted_node_);
  }

  // Return true for your app to continue running, false to terminate the app.
  return true;
}

bool PathfinderDemo::eventListener(lumin::ServerEvent* event) {
  // Place your event handling here.
  lumin::ServerEventTypeValue typ = event->getServerEventTypeValue();
  if (typ == lumin::ControlPose6DofInputEventData::GetServerEventTypeValue()) {
    requestWorldRayCast(getHeadposeWorldPosition(), getHeadposeWorldForwardVector(), 0);
    return false;
  } else if (typ == lumin::RayCastEventData::GetServerEventTypeValue()) {
    lumin::RayCastEventData* raycast_event = static_cast<lumin::RayCastEventData*>(event);
    std::shared_ptr<lumin::RaycastResult> raycast_result = raycast_event->getHitData();
    switch (raycast_result->getType()) {
      case lumin::RaycastResultType::kQuadNode: {
        std::shared_ptr<lumin::RaycastQuadNodeResult> quad_result = std::static_pointer_cast<lumin::RaycastQuadNodeResult>(raycast_result);
        focus_node_ = quad_result->getNodeId();
        return false;
      }
      default: {
        focus_node_ = lumin::INVALID_NODE_ID;
        return false;
      }
    }
  } else if (typ == lumin::GestureInputEventData::GetServerEventTypeValue()) {
    lumin::GestureInputEventData* gesture_event = static_cast<lumin::GestureInputEventData*>(event);
    switch (gesture_event->getGesture()) {
      case lumin::input::GestureType::TriggerClick: {
        return onClick();
      }
      default: {
        return false;
      }
    }
  } else {
    return false;
  }
}

bool PathfinderDemo::onClick() {
  lumin::RootNode* root_node = prism_->getRootNode();
  for (int i=0; i<NUM_QUADS; i++) {
    lumin::Node* node = prism_->findNode(QUAD_NAMES[i], root_node);
    if (node->getNodeId() == focus_node_) {
      dispatch(SVG_NAMES[i]);
      return true;
    }
  }
  return false;
}

void PathfinderDemo::dispatch(const char* svg_filename) {
   ML_LOG(Info, "Dispatching %s", svg_filename);

   MLDispatchPacket* dispatcher;
   if (MLResult_Ok != MLDispatchAllocateEmptyPacket(&dispatcher)) {
     ML_LOG(Error, "Failed to allocate dispatcher");
     return;
   }

   if (MLResult_Ok != MLDispatchAllocateFileInfoList(dispatcher, 1)) {
     ML_LOG(Error, "Failed to allocate file info list");
     return;
   }

   MLFileInfo* file_info;
   if (MLResult_Ok != MLDispatchGetFileInfoByIndex(dispatcher, 0, &file_info)) {
     ML_LOG(Error, "Failed to get file info");
     return;
   }

   if (MLResult_Ok != MLFileInfoSetFileName(file_info, svg_filename)) {
     ML_LOG(Error, "Failed to set filename");
     return;
   }
   
   if (MLResult_Ok != MLFileInfoSetMimeType(file_info, "image/svg")) {
     ML_LOG(Error, "Failed to set mime type");
     return;
   }

   if (MLResult_Ok != MLDispatchAddFileInfo(dispatcher, file_info)) {
     ML_LOG(Error, "Failed to add file info");
     return;
   }

   MLResult result = MLDispatchTryOpenApplication(dispatcher);
   if (MLResult_Ok != result) {
     ML_LOG(Error, "Failed to dispatch: %s", MLDispatchGetResultString(result));
     return;
   }

   // https://forum.magicleap.com/hc/en-us/community/posts/360043198492-Calling-MLDispatchReleaseFileInfoList-causes-a-dynamic-link-error
   // if (MLResult_Ok != MLDispatchReleaseFileInfoList(dispatcher, false)) {
   //   ML_LOG(Error, "Failed to deallocate file info list");
   //   return;
   // }
   
   if (MLResult_Ok != MLDispatchReleasePacket(&dispatcher, false, false)) {
     ML_LOG(Error, "Failed to deallocate dispatcher");
     return;
   }
}

extern "C" void init_scene_thread(uint64_t id) {}

