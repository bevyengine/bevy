---
title: Methods of `Affine3` `to_transpose` and `inverse_transpose_3x3` are now part of an extension trait
pull_requests: [22681]
---

With the addition of `Affine3` on glam, Bevy's version was removed. To keep the functionality that Bevy's version provided we created the extension trait `Affine3Ext`. Locations that accessed  `Affine3::to_transpose` or `Affine3::inverse_transpose_3x3` will now need the extension trait to be in scope.
