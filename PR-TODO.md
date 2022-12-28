# TODO

Just a dev todo, delete file eventually.

* UniformComponentPlugin a sprite uniform
* In extract_sprites (see extract_meshes), add uniform as component
* SpriteUniform, component + shadertype + clone, field: entity_index: u32
* Need to add the bind group in SpritePipeline (?) see MeshPipeline's FromWorld
* Then in queue_sprite_bind_group (?) see queue_mesh_bind_group we need Res<ComponentUniforms<SpriteUniform>>
	- Create the bind group and clone the binding here
	- Insert the resource
* Then we need something like SetMeshBindGroup
* Then update the sprite draw function

* Lastly, remove all the vertex based sprite stuff
