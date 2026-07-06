---
title: "Elliptical Border Radius"
authors: ["@ickshonpe"]
pull_requests: [24779]
---

Bevy UI now supports Nodes with elliptical border geometry. 

The fields of `BorderRadius` are now `Val2`s to enable different radii to be set for each axis.

`BorderRadius` has new elliptical constructor functions:
- `elliptical`: set indivdual elliptical radii for each corner.
- `all_elliptical`: set all four corners to the same elliptical radii.


