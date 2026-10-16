---
title: "`AutoExposure` pre-exposes the view"
pull_requests: [26211]
---

A camera with `AutoExposure` now renders with its adapted exposure, read back from the GPU,
instead of its `Exposure` component. `view.exposure` in shaders, and `ExtractedCamera::exposure`
in the render world after `RenderSystems::CreateViews`, hold the adapted exposure and change as
auto exposure adapts.

Light that shaders scale by `view.exposure` displays as before, up to `HDR_CEILING` (see the
migration guide for the HDR ceiling). Colors written without it, such as unlit materials and the
clear color, are no longer adjusted by auto exposure once the view is pre-exposed, apart from small
shifts while the exposure changes. To render with the camera's `Exposure` as before, set
`AutoExposure::pre_exposure` to `false`.

Code that scales values it writes into the main texture by the camera's `Exposure` should use
`view.exposure`, or `ExtractedCamera::exposure` after `RenderSystems::CreateViews`, instead.
Custom temporal effects that keep a history of the main texture should rescale it by the ratio of
`view.exposure` to the exposure it was stored at.

The skybox shader now multiplies `Skybox::brightness` by `view.exposure`, so the render world's
`SkyboxUniforms::brightness` no longer includes the camera's exposure.

`AutoExposure` has two new fields, `pre_exposure` (default `true`) and `reset` (default `false`).
If you construct `AutoExposure` without `..Default::default()`, add both fields.
