---
title: "`EnvironmentMapUniform` is removed"
pull_requests: [24095]
---

`EnvironmentMapUniform` has been removed. It previously stored the rotation transformation matrix of view environment maps. Now the rotation is stored as a quaternion in `LightProbesUniform::view_rotation`.
