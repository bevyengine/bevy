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
    * Add runtime type safety to uniform bindings (and maybe compile time)
    * Inject layout set/bindings into shader source so they don't need to be defined in-shader
* Error Handling
    * Custom error type?
    * Remove as many panics / unwraps as possible
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