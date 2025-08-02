---
title: `AmbientLight` deprecated
pull_requests: [18207]
---

`AmbientLight`s have been deprecated in favor of using `EnvironmentMapLight`s for the same purpose.
All usages of an ambient light can be replaced by `EnvironmentMapLight::solid_color` added to the camera.
This will render slightly differently, as previously, `AmbientLight`s were (incorrectly) treated as diffuse-only light sources, 
while `EnvironmentMapLight`s have both a specular and diffuse component.
