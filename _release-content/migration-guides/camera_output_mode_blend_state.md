---
title: "`CameraOutputMode::Write::blend_state` behavior change"
pull_requests: [24452]
---

Previously `CameraOutputMode::Write::blend_state` defaulted to `None`. And when set to `None` the first camera
for the render target will disable blending (overwrite existing data of the texture) and subsequent cameras will use alpha blending.

Now `CameraOutputMode::Write::blend_state` has been changed to default to `BlendState::ALPHA_BLENDING`, and the `blend_state` is used as-is,
which means `None` will always disable blending instead of applying the above logic.

If you want the original behavior, you can manually set the `CameraOutputMode::Write::blend_state` of the first camera to `None`
and subsequent cameras to `BlendState::ALPHA_BLENDING`.
