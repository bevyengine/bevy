---
title: "Add an infinite grid"
authors: [ "@icesentry" ]
pull_requests: [ 23482 ]
---

# Add an infinite grid to bevy_dev_tools

When working on a 3d scene in an editor it's often very useful to have a transparent grid that indicates the ground plane and the major axis.

There are various techniques to render an infinite grid and avoid artifacts. 
This implementation works by rendering the grid as a fullscreen shader. 
The grid is rendered from the perspective of the camera and fades out relative to the camera position. 
The fade out hides artifacts from drawing lines too far in the horizon.

