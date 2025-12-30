---
title: "Userspace glTF Extension Handling"
authors: ["@christopherbiscardi"]
pull_requests: [22106]
---

Prior to 0.18, the code to handle extensions like `KHR_lights_punctual` was hardcoded into Bevy's glTF loader.
In 0.18, users may implement the `GltfExtensionHandler` trait to do stateful processing of glTF data as it loads.
Processing _extension_ data is only half the story here because to process extension data you also have to be able to process the non-extension data like meshes, materials, animations, and more.

Extension handlers can be written for wide variety of use cases, including:

- Insert Bevy Component data on entities
- Convert all `Mesh3d` components to `Mesh2d`
- Build `AnimationGraph`s and insert them on animation roots
- Replace `StandardMaterial` with custom materials
- Insert lightmaps

## Extras vs Extensions

glTF has two mechanisms for extending glTF files with additional user data: Extras and Extensions.

**Extras** are meant to be arbitrary application-specific data, often authored by users directly in tools like Blender's custom properties.
Extras are historically well supported by Bevy; If you add a custom property in Blender that data will end up in one of the `GltfExtras` components on the relevant `Entity`.

**Extensions** are meant for data that can be shared across applications.
They are more flexible, allowing for new data in more places inside a glTF file, and more powerful as a result.
Extensions can add new object types, such as `lights` from the `KHR_lights_punctual` extension, as well as arbitrary buffers, data that is at the root of the glTF file, and more.

More examples of extensions can be found in the [KhronosGroup git repo](https://github.com/KhronosGroup/glTF/blob/7bbd90978cad06389eee3a36882c5ef2f2039faf/extensions/README.md)

## Case Study

Extensions typically require an application that is _producing_ the data as well as _consuming_ the data.

For example: [Skein](https://github.com/rust-adventure/skein) defines a glTF extension that allows adding Bevy Components to glTF objects.
This is most commonly produced by Blender and consumed by Skein's `GltfExtensionHandler` in Bevy.
These components are then inserted on entities in a scene at the same time built-in components like `Transform` and `Mesh3d` are.

Using glTF Extensions for this data means that other level editors like Trenchbroom can also write the same format to glTF files.
Any third party software that writes component data into a glTF file can use Skein's `GltfExtensionHandler`, resulting in components being "ready-to-go" when spawning `Scene`s.

## New Examples

Two new examples show off use cases:

- The first builds an `AnimationGraph` and inserts it onto the animation root in a Scene, which means it is now accessible to play animations using the `AnimationPlayer` on the same `Entity` later when that Scene is spawned.
- The second uses a `GltfExtensionHandler` to switch the 3d Mesh and Material components for their 2d counterparts. This is useful if you're using software like Blender to build 2d worlds.

```shell
cargo run --example gltf_extension_animation_graph
cargo run --example gltf_extension_mesh_2d
```
