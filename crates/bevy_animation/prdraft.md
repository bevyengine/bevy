Property based Animation PR (draft)

Any type that implements `bevy_reflect::Struct` can have their attributes animated.

Has a safety restriction only attributes that are within the struct memory limits can be addressed and animated,
this means you can't animate items behind a `Vec`, array types could be animated but nested properties implementation
is missing.

The system main goal is to provide an low level api to play and blend animations.
That been said an animation state machine should be built by the user or provided by a external crate.

## Features

1. Property based animation

```rust
let mut clip = Clip::default();
/// A property is defined by the entity path followed by the component property
clip.add_curve_at_path("/Node1/Node2@Transform.translation", track_data);
```

2. Animate any type using bevy_reflect

```rust
#[derive(Debug, Reflect)]
#[reflect(Component)]
struct CustomComponent {
    value: f32,
    point: Vec3,
    #[reflect(ignore)]
    non_animated: Vec3,
}

// During app startup
app_builder.register_animated_component::<bevy_transform::prelude::Transform>();

let mut clip = Clip::default();
clip.add_curve_at_path("@CustomComponent.value", ...);
clip.add_curve_at_path("@CustomComponent.point", ...);
```

4. Animate assets by swapping handles or animate their properties

```rust
// During app startup
app_builder.register_animated_asset::<bevy_pbr::prelude::StandardMaterial>();

let mut clip = Clip::default();
// Swap material with
clip.add_curve_at_path("@Handle<StandardMaterial>", ...);
// Change the current material properties
clip.add_curve_at_path("@Handle<StandardMaterial>.albedo", ...);
clip.add_curve_at_path("@Handle<StandardMaterial>.albedo_texture", ...);
```

5. GLTF support

You can load any animation clips embedded within a GLTF file

```rust
// Loads the skinned mesh ready to be animated with all of its clips (if any)
let char_scene_handle = asset_server.load("models/character_medium/character_medium.gltf");
// Returns a single clip from the GLTF
let clip_handle = asset_server.load("models/character_medium/idle.gltf#Anim0");
```

6. Animation Blending

It's possible to blend `bool`, `f32`, `Vec2`, `Vec3`, `Vec4`, `Quat`, `Handle<T>`, `Option<T>` when `T` can also be blended, and more.

7. Animation clips defined by a named hierarchy

Each `Animator` can only animate children entities, this is similar to other engines like Unity;

Each clip have it's own hierarchy space, this allows clips to be reused for different setups that
share some similarities. 

Note that this isn't a humanoid retargeting system, but it could give you some good results is some particular situations;

8. Mostly safe :p

This crate uses a couple `unsafe`'s blocks, the safety rules of these blocks should be guaranteed
at within the scope of a function or file (in the worst case) this way changes can be more easily
made without the fear braking random safety conditions, as a bonus they should be asserted
by code whenever possible;

**The unsafe code still needs a review** 

9. Fast TM

a) Each component animation runs in parallel from each other
b) Keyframe cursor, allows for faster sample rates by remembering the last keyframe it was last frame
c) NLerp only and you wont notice the difference, hopefully ...

10. Skinned Mesh support out of the box

Simple and naive implementation, replaces the vertex shader from the `bevy_pbr::prelude::StandardMaterial`.
Further work should done along side with the rendering, but hey it works!  

11. Additive Blending

Supports additive blending as defined by the ACL mode Additive 1, that can be found
[here](https://github.com/nfrechette/acl/blob/develop/docs/additive_clips.md)

12. SIMD support

Clips can now be packed, a packed clip will contain tracks that are sampled using wide types from the `ultraviolet` crate
thus resulting in a greater sampling throughput.

## Missing Features

1. Nested properties

Right now only the top most properties of each struct can be animated,
this means `"@CustomComponent.point.x"` isn't valid;

2. Masks

3. Animation events (simple and interval)

4. Morph Targets

5. Dedicated `Transform` animator system

`Transform` is the most common animated component in any game, its the only animated component registered
by default and it should receive a custom animator system to squeeze every single us that is possible,
this custom system should be also be capable of performing SIMD blending;


## Terminology and Internals

A clip is where all the animation data is stored, it's made out of a hierarchy of entities
and a list of animated properties, each property is defined by path (`Transform.translation`) and type (`Vec3`)
and contains a list of all the samplers paired with their respective output entities or channels;

A sampler evaluates the state of a property at any given time. The `Track` is the only sampler currently implemented
and comes in 3 flavors.

1. `TrackFixed` performant in any operation range, requires a keyframe for every frame of the animation duration;

2. `TrackVariableLinear`, can be used with linear keyframe reduction technique to save memory at runtime,
most performant when sampled at a higher fps than the track sample rate. 

3. `TrackVariable` very similar to `TrackVariableLinear` but beyond linear it supports step and catmull-rom as interpolations methods,
because of that it will have a lower performance and require and extra runtime memory to store the keyframes tangents;
This track is similar to `AnimationCurve` from Unity engine;

## Safety status

Assume that `bevy_reflect::Struct` was correctly implemented

For that to happen unsafe blocks are used, but their safety constrains should be
self contained within the same file or even with the same function block, with one
single exception.

Asside from that an animator system can be implemented with only safe code for any
type that you may want, its a lot of code but mostly is boiler plates. Optionally you can
add two extra unsafe blocks to improve performance (with very little safety conditions);