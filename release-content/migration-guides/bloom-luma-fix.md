---
title: "Bloom luma calculation now in linear space"
pull_requests: [22561]
---

The luma calculation for bloom's Karis average (used for downsampling) has been corrected to use linear color space instead of non-linear sRGB space.

As a result, the intensity of the bloom effect may appear reduced, especially for colors with high saturation or those that were significantly affected by the previous non-linear calculation.

If your scene's bloom now appears too dim, you can:

- Increase the `intensity` field on the `Bloom` component.
- Increase the `emissive` strength of your materials.
- Adjust the `prefilter` settings in the `Bloom` component.
