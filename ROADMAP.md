# Bevy Roadmap

Here is the current list of planned features. All items are sorted in approximate priority order, but actual implementation order will vary based on individual interest and/or funding.

* UI Framework
    * Text
    * Styling
* Rendering
    * Textures
    * Physically based rendering
    * Skeletal animation
    * Macro to produce vertex buffer attributes (and maybe descriptors) from structs
    * Dynamic / user defined shaders
        * consider using shaderc-rs. but this introduces compile complexity and requires other C++ build systems
* Input
    * Keyboard and mouse events
    * Gamepad events
* Assets
    * Load GLTF files
* Scene
    * Define scene format
    * Load scenes from files (likely RON)
* Plugins
    * Live plugin reloading
* Editor
    * Editor <-> game communication protocol
    * Build UI using bevy UI framework
    * Consider supporting embedding parts of the editor directly into games
* Physics
    * High level physics data types
    * Integrate with nphysics
* Platform Support
    * Android
    * iOS
    * Web