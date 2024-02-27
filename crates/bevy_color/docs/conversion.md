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

For example, the [sRGB](Srgba) space is defined by its relationship with [Linear RGB](LinearRgba), and [HWB](Hwba) by its with [sRGB](Srgba). As such, it is the responsibility of [sRGB](Srgba) to provide [From] implementations for [Linear RGB](LinearRgba), and [HWB](Hwba) for [sRGB](Srgba). To then provide conversion between [Linear RGB](LinearRgba) and [HWB](Hwba) directly, [HWB](Hwba) is responsible for implementing these conversions, delegating to [sRGB](Srgba) as an intermediatory. This ensures that all conversions take the shortest path between any two spaces, and limit the proliferation of domain specific knowledge for each color space to their respective definitions.
