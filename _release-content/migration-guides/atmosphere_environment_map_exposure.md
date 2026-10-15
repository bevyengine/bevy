---
title: "`AtmosphereEnvironmentMapLight` cubemaps are pre-exposed"
pull_requests: []
---

The cubemap generated for `AtmosphereEnvironmentMapLight` samples the pre-exposed sky-view LUT, so
it stores radiance multiplied by the exposure of the view that rendered it. The new render world
resource `EnvironmentMapExposures` records that exposure for each image. Bevy's image-based
lighting and Solari divide it back out, so `EnvironmentMapLight::intensity` works as before.

Code that samples the atmosphere's cubemap directly, or a map filtered from it by
`GeneratedEnvironmentMapLight`, must divide by `EnvironmentMapExposures::get` for that image.
