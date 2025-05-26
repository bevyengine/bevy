---
title: `ImageNode` rotation
authors: ["@hukasu"]
pull_requests: [19340]
---

It is now possible to rotate an `ImageNode`.  
Rotations using `ImageNode::rotation` do not update
the `Node`'s computed size and might lead to clipping or overflow,
for rotations that update the `Node`'s computed size use `Transform`.

```rust
// This does not update the node's computed size 
commands.spawn((
    ImageNode {
        image: handle.clone(),
        rotation: FRAC_PI_2,
        ..default()
    }
));

// This does update the node's computed size 
commands.spawn((
    ImageNode {
        image: handle.clone(),
        ..default()
    },
    Transform::from_rotation(Quat::from_rotation_z(FRAC_PI_2))
));
```
