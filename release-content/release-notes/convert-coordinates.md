---
title: Allow importing glTFs with a corrected coordinate system
authors: ["@janhohenheim"]
pull_requests: [19633]
---

glTF uses the following coordinate system:

- forward: Z
- up: Y
- right: -X

and Bevy uses:

- forward: -Z
- up: Y
- right: X

This means that to correctly import glTFs into Bevy, vertex data should be rotated by 180 degrees around the Y axis.  
For the longest time, Bevy has simply ignored this distinction. That caused issues when working across programs, as most software respects the
glTF coordinate system when importing and exporting glTFs. Your scene might have looked correct in Blender, Maya, TrenchBroom, etc. but everything would be flipped when importing it into Bevy!

Long-term, we'd like to fix our glTF imports to use the correct coordinate system by default.
But changing the import behavior would mean that *all* imported glTFs of *all* users would suddenly look different, breaking their scenes!
Not to mention that any bugs in the conversion code would be incredibly frustating for users.

This is why we are now gradually rolling out support for corrected glTF imports. Starting now you can opt into the new behavior by setting the `GltfLoaderSettings`:

```rust
// old behavior, ignores glTF's coordinate system
let handle = asset_server.load("fox.gltf#Scene0");

// new behavior, converts glTF's coordinate system into Bevy's coordinate system
let handle = asset_server.load_with_settings(
    "fox.gltf#Scene0",
    |settings: &mut GltfLoaderSettings| {
        settings.convert_coordinates = true;
    },
);
```

Afterwards, your scene will oriented such that your modeling software's forward direction correctly corresponds to Bevy's forward direction.

For example, Blender assumes -Y to be forward, so exporting the following model to glTF and loading it in Bevy with the new settings will ensure everything is
oriented the right way across all programs in your pipeline:

<!-- TODO: Add png from PR description -->
![Blender Coordinate System](blender-coords.png)

If you opt into this, please let us know how it's working out! Is your scene looking like you expected? Are the animations playing correctly? Is the camera at the right place? Are the lights shining from the right spots?
