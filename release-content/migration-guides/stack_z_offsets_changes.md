---
title: Fixed UI draw order and `stack_z_offsets` changes
pull_requests: [19691]
---

The draw order of some renderable UI elements relative to others wasn't fixed and depended on system ordering.
In particular the ordering of background colors and texture sliced images was sometimes swapped.

The UI draw order is now fixed.
The new order is (back-to-front):

1. Box shadows

2. Node background colors

3. Node borders

4. Gradients

5. Border Gradients

6. Images (including texture-sliced images)

7. Materials

8. Text (including text shadows)

The values of the `stack_z_offsets` constants have been updated to enforce the new ordering. Other changes:

* `NODE` is renamed to `BACKGROUND_COLOR`

* `TEXTURE_SLICE` is removed, use `IMAGE`.

* New `BORDER`, `BORDER_GRADIENT` and `TEXT` constants.
