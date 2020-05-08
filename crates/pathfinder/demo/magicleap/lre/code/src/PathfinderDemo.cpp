// %BANNER_BEGIN%
// ---------------------------------------------------------------------
// %COPYRIGHT_BEGIN%
//
// Copyright (c) 2018 Magic Leap, Inc. All Rights Reserved.
// Use of this file is governed by the Creator Agreement, located
// here: https://id.magicleap.com/creator-terms
//
// %COPYRIGHT_END%
// ---------------------------------------------------------------------
// %BANNER_END%

// %SRC_VERSION%: 1

#include <PathfinderDemo.h>
#include <lumin/node/RootNode.h>
#include <lumin/ui/Cursor.h>
#include <ml_logging.h>
#include <scenes.h>
#include <PrismSceneManager.h>

PathfinderDemo::PathfinderDemo() {
  ML_LOG(Debug, "PathfinderDemo Constructor.");

  // Place your constructor implementation here.
}

PathfinderDemo::~PathfinderDemo() {
  ML_LOG(Debug, "PathfinderDemo Destructor.");

  // Place your destructor implementation here.
}

const glm::vec3 PathfinderDemo::getInitialPrismSize() const {
  return glm::vec3(2.0f, 2.0f, 2.0f);
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
  lumin::ui::Cursor::SetScale(prism_, 0.03f);
  spawnInitialScenes();

  // Place your initialization here.

  return 0;
}

int PathfinderDemo::deInit() {
  ML_LOG(Debug, "PathfinderDemo Deinitializing.");

  // Place your deinitialization here.

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

  // Return true for your app to continue running, false to terminate the app.
  return true;
}

bool PathfinderDemo::eventListener(lumin::ServerEvent* event) {

  // Place your event handling here.

  // Return true if the event is consumed.
  return false;
}

