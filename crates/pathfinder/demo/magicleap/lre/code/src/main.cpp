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
#include <ml_logging.h>

int main(int argc, char **argv)
{
  ML_LOG(Debug, "PathfinderDemo Starting.");
  PathfinderDemo myApp;
  return myApp.run();
}

