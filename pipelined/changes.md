* Remove AppBuilder
* Add SubApps
* `Res<Box<dyn RenderResourceContext>>` -> `Res<RenderResources>`
* Removed RenderResourceBindings
* Made shaders and pipelines proper render resources (removes dependency on bevy_asset and is generally a cleaner api)
* Removed RenderResources / RenderResource traits
* Decoupled swap chain from Window in Renderer api
* Removed RenderResourceBindings
* Removed asset tracking from render resources
* Removed cruft from RenderResource api