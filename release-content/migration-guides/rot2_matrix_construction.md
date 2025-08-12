---
title: "Fix `From<Rot2>` implementation for `Mat2`"
pull_requests: [20522]
---

Past releases had an incorrect `From<Rot2>` implementation for `Mat2`, constructing a rotation matrix in the following form:

```text
[  cos, sin ]
[ -sin, cos ]
```

This was actually the *inverse* of the rotation matrix, resulting in clockwise rotation when transforming vectors.
The correct version is the following:

```text
[ cos, -sin ]
[ sin,  cos ]
```

resulting in counterclockwise rotation.

This error has now been fixed. You may see that rotation matrices created using the `From<Rot2>` implementation
produce different results than before, as the rotation happens counterclockwise (as intended) rather than clockwise.
Invert either the input `Rot2` or the resulting `Mat2` to get the same results as before.
