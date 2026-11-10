---
title: Number input interior labels
authors: ["@viridia"]
pull_requests: []
---

`FeathersNumberInput` can now show its label inside the field, with the label on the left and the value on the right.
This saves space in dense layouts such as the property fields in Blender's node graph editor.
The label is hidden while the value is being edited.

```rust
bsn! {
    @FeathersNumberInput { @interior_label_text: "Roughness" }
    NumericValue::F32(0.5)
}
```
