---
title: Methods of `Affine3` `to_transpose` and `inverse_transpose_3x3` are now part of an extension trait
pull_requests: [22681]
---

With the addition of `Affine3` on glam, Bevy's version was removed, to keep the functionalities that
the Bevy version provided the extension trait `Affine3Ext` was created. Locations that used to
access `Affine3::to_transpose` or `Affine3::inverse_transpose_3x3` will now require that the
extension be in scope.
