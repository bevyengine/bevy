---
title: "Userspace glTF Extension Handling"
authors: ["@christopherbiscardi"]
pull_requests: [22106]
---

glTF has two mechanisms for extending glTF files with additional user data: Extras and Extensions.

**Extras** are meant to be arbitrary application-specific data, often authored by users directly in tools like Blender's custom properties.
Extras are historically well supported by Bevy; If you add a custom property in Blender that data will end up in one of the `GltfExtras` components on the relevant `Entity`.

**Extensions** are meant for data that can be shared across applications.
They are more flexible, allowing for new data in more places inside a glTF file, and more powerful as a result.
For example, there are a few object types in a glTF file including meshes, nodes, scenes, and more.
Extensions can add new object types, such as `lights` from the `KHR_lights_punctual` extension, as well as arbitrary buffers, data that is at the root of the glTF file, and more.

More examples of extensions can be found in the [KhronosGroup git repo](https://github.com/KhronosGroup/glTF/blob/7bbd90978cad06389eee3a36882c5ef2f2039faf/extensions/README.md)

## GltfExtensionHandler

Having extension data in a glTF file isn't enough; There also needs to be an application _producing_ the data as well as _consuming_ the data.
Prior to 0.18, extension handling for extensions like `KHR_lights_punctual` was hardcoded into the glTF loader.
In 0.18, users implement the `GltfExtensionHandler` trait to do stateful processing of glTF data.
Hooks are called during the loading process, from the glTF loader, which allows for modifying Scene entities, creating additional new assets, and more.

[Skein](https://github.com/rust-adventure/skein), which is most well known for its Blender/Bevy integration addon, defines a glTF extension that allows adding Bevy Components to glTF objects.
These components are then inserted on entities in a scene at the same time built-in components like `Transform` and `Mesh3d` are.

Using glTF Extensions for this data means that other level editors like Trenchbroom can also write the same format.
Any third party software that writes component data into a glTF file can use Skein's `GltfExtensionHandler`, resulting in components being "ready-to-go" when spawning `Scene`s.

Processing glTF Extension data is only half the story because to process Extension data you also have to be able to process the non-extension data.
Two new examples show off use cases:

- The first builds an `AnimationGraph` and inserts it onto the animation root in a Scene, which means it is now accessible to play animations using the `AnimationPlayer` on the same `Entity` later when that Scene is spawned.
- The second uses a `GltfExtensionHandler` to switch the 3d Mesh and Material components for their 2d counterparts. This is useful if you're using software like Blender to build 2d worlds.

```shell
cargo run --example gltf_extension_animation_graph
cargo run --example gltf_extension_mesh_2d
```
