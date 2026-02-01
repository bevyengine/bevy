# Conversion

Conversion between the various color spaces is achieved using Rust's native [From] trait. Because certain color spaces are defined by their transformation to and from another space, these [From] implementations reflect that set of definitions.

```rust
# use bevy_color::{Srgba, LinearRgba};
let color = Srgba::rgb(0.5, 0.5, 0.5);

// Using From explicitly
let linear_color = LinearRgba::from(color);

// Using Into
let linear_color: LinearRgba = color.into();
```

For example, the [sRGB](crate::Srgba) space is defined by its relationship with [Linear RGB](crate::LinearRgba), and [HWB](crate::Hwba) by its with [sRGB](crate::Srgba). As such, it is the responsibility of [sRGB](crate::Srgba) to provide [From] implementations for [Linear RGB](crate::LinearRgba), and [HWB](crate::Hwba) for [sRGB](crate::Srgba). To then provide conversion between [Linear RGB](crate::LinearRgba) and [HWB](crate::Hwba) directly, [HWB](crate::Hwba) is responsible for implementing these conversions, delegating to [sRGB](crate::Srgba) as an intermediatory. This ensures that all conversions take the shortest path between any two spaces, and limit the proliferation of domain specific knowledge for each color space to their respective definitions.
