# Conversion

Conversion between the various color spaces is achieved using Rust's native [From] trait.
Because certain color spaces are defined by their transformation to and from
another space, these [From] implementations reflect that set of definitions.

```rust
# use bevy_color::{Srgba, LinearRgba};
let color = Srgba::rgb(0.5, 0.5, 0.5);

// Using From explicitly
let linear_color = LinearRgba::from(color);

// Using Into
let linear_color: LinearRgba = color.into();
```

For example, the [sRGB] space is defined by its relationship with [Linear RGB], and [HWB] by its with [sRGB].
As such, it is the responsibility of [sRGB] to provide [From] implementations for [Linear RGB], and [HWB] for [sRGB].
To then provide conversion between [Linear RGB] and [HWB] directly, [HWB] is responsible
for implementing these conversions, delegating to [sRGB] as an intermediatory.
This ensures that all conversions take the shortest path between any two spaces,
and limit the proliferation of domain specific knowledge for each color space to their respective definitions.

[sRGB]: <crate::Srgba>
[Linear RGB]: <crate::LinearRgba>
[HWB]: <crate::Hwba>
