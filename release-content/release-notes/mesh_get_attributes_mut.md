---
title: Allow mutable access to multiple Mesh attributes simultaneously
authors: ["@hukasu"]
pull_requests: [18976]
---

`Mesh` holds multiple attributes, having mutable access to multiple attributes
simultaneously allows to edit them all at once, if they are derived from the same data.

To access multiple the attributes you can use the new method `get_attributes_mut`:

```rust
let [pos1, pos2, pos3, normal, color] = mesh.get_attributes_mut([
    &Mesh::ATTRIBUTE_POSITION.id,
    &Mesh::ATTRIBUTE_POSITION.id,
    &Mesh::ATTRIBUTE_POSITION.id,
    &Mesh::ATTRIBUTE_NORMAL.id,
    &Mesh::ATTRIBUTE_COLOR.id,
]);
assert_eq!(pos1.unwrap().0, &Mesh::ATTRIBUTE_POSITION);
assert!(pos2.is_none());
assert!(pos3.is_none());
assert_eq!(normal.unwrap().0, &Mesh::ATTRIBUTE_NORMAL);
assert_eq!(color.unwrap().0, &Mesh::ATTRIBUTE_COLOR);
```

Passing an attribute multiple times returns a mutable reference on the first instance
and `None` on all following instances.
