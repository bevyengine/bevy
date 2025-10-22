---
title: Ring primitives
authors: ["@tigregalis", "@lynn-lumen"]
pull_requests: [21446]
---

## Ring / hollow shapes

![Rings of 2d primitives (bottom row)](https://github.com/user-attachments/assets/8fac6c82-3da0-488e-ab38-80816b2129c0)

![Extrusions of rings of extrudable primitives (front row)](https://github.com/user-attachments/assets/70c4dee0-4f82-4723-b95c-9d02ddb95363)

There is a new generic primitive `Ring`, which takes as input any `Primitive2d`, with two instances of that primitive shape: the outer and the inner (or hollow).
A `Ring` here is what an `Annulus` is to a `Circle`.
This allows us to have (or at least approximate - more on that later) "hollow" shapes or "outlines".

```rs
// construct the `Ring` from an outer and inner shape

let capsule_ring = Ring::new(Capsule2d::new(50.0, 100.0), Capsule2d::new(45.0, 100.0));
let hexagon_ring = Ring::new(RegularPolygon::new(50.0, 6), RegularPolygon::new(45.0, 6)); // note vertex count must match

// or, from a shape and a thickness for any types that implement `Inset`

let capsule_ring = Ring::from_primitive_and_thickness(Capsule2d::new(50.0, 100.0), 5.0);
let hexagon_ring = Ring::from_primitive_and_thickness(RegularPolygon::new(50.0, 6), 5.0);

// or, from the `ToRing` trait for any types that implement `Inset`

let capsule_ring = Capsule2d::new(50.0, 100.0).to_ring(5.0);
let hexagon_ring = RegularPolygon::new(50.0, 6).to_ring(5.0);

```

## How it works

The mesh for a `RingMeshBuilder` is constructed by concatenating the vertices of the outer and inner meshes, then walking the perimeter to join corresponding vertices like so:

![Vertices around a pentagon ring](https://github.com/user-attachments/assets/2cecb458-3b59-44fb-858b-1beffecd1e57)

```text
# outer vertices, then inner vertices
positions = [
  0  1  2  3  4
  0' 1' 2' 3' 4'
]
# pairs of triangles
indices = [
  0  1  0'    0' 1  1'
  1  2  1'    1' 2  2'
  2  3  2'    2' 3  3'
  3  4  3'    3' 4  4'
  4  0  4'    4' 0  0'
]
```

Examples of generated meshes:

![Mesh for a pentagon ring](https://github.com/user-attachments/assets/cb9881e5-4518-4743-b8de-5816b632f36f)

![Mesh for a heart ring](https://github.com/user-attachments/assets/348bbd91-9f4e-4040-bfa5-d508a4308c10)

## Extrusions

A `Ring` for a type that is `Extrudable` is also `Extrudable`.

```rs
let extrusion = Extrusion::new(RegularPolygon::new(1.0, 5).to_ring(0.2));
```

![Mesh for an extruded pentagon ring](https://github.com/user-attachments/assets/7d2022c9-b8cf-4b4b-bb09-cbe4fe49fb89)

![Mesh for an extruded heart ring](https://github.com/user-attachments/assets/dbaf894e-6f7f-4b79-af3e-69516da85898)

## Inset shapes

Some shapes can be "inset", that is, we can produce a smaller shape where the lines/curves/vertices are equidistant from the outer shape's when they share the same origin.
This is represented by the `Inset` trait.
Inset shapes give us nice "outlines" when combined with `Ring`, so for these shapes we provide a `ToRing` method that takes an inset distance.

The implementation of `Inset` can be unintuitive - have a look at the source at [crates/bevy_math/src/primitives/inset.rs][Source].
For example, the inset `CircularSegment` in our implementation is actually constructed by shortening the radius _and_ the angle.

Some shapes can't be represented by an inset: `Ellipse` for example doesn't implement `Inset`, because concentric ellipses do not have parallel lines.

![Concentric ellipses](https://github.com/user-attachments/assets/3f419f8f-4d7a-4bfb-a231-fba9464e0f93)

If the ellipse is not a circle, the inset shape is not actually an ellipse (although it may look like one) but can also be a lens-like shape.
The following image shows an ellipse in white and all points at a constant distance from that ellipse in blue.
Neither of the blue shapes is an ellipse.

![An ellipse in white and its parallel lines in blue](https://github.com/user-attachments/assets/8c7520d1-9911-4c9c-8e6f-2688e160f510)

For the sake of flexibility, however, we don't require `Ring` shapes to be `Inset`.

## Limitations

It's assumed that the inner and outer meshes have the same number of vertices.

It's currently assumed the vertex positions are well ordered (i.e.
walking around the perimeter, without zig-zagging), otherwise it will result in incorrect geometries.

The `outer_shape` must contain the `inner_shape` for the generated meshes to be accurate.
If there are vertices in the `inner_shape` that escape the `outer_shape` (for example, if the `inner_shape` is in fact larger), it may result in incorrect geometries.

Because the origin of the generated mesh matters when constructing a `Ring`, some "outline" shapes can't currently be easily represented.

<!-- TODO: Update link -->

[Source]: https://github.com/bevyengine/bevy/blob/6e348948cae9523d0d7f13f0ed598d16790ff4ae/crates/bevy_math/src/primitives/inset.rs
