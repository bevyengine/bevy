---
title: "Atmosphere lookup textures are pre-exposed and store optical depth"
pull_requests: []
---

The atmosphere's lookup textures (LUTs) store different values, which affects custom shaders that
read them:

- The sky-view and aerial-view LUTs store luminance multiplied by the view's exposure.
  `sample_sky_view_lut` and `sample_aerial_view_lut` still return absolute luminance. Code that
  samples the textures itself must divide by `view.exposure`.
- The transmittance LUT stores optical depth instead of transmittance. `sample_transmittance_lut`
  still returns transmittance, and `sample_optical_depth_lut` returns the stored depth. Code that
  samples the texture itself must take `exp(-depth)`.
- The multiscattering LUT stores the natural log of the multiscattering factor.
  `sample_multiscattering_lut` still returns the factor.
