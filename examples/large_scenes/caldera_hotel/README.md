# Caldera Hotel 01 Example

Currently only setup to load `hotel_01`.

Download scene from https://github.com/Activision/caldera

Reexport `map_source/prefabs/br/wz_vg/mp_wz_island/commercial/hotel_01.usd` as `hotel_01.glb`

[Alternate processed files applied animation base poses (glTF files on discord)](https://discord.com/channels/691052431525675048/1159383661062389790/1283123002346705018)

When importing the USD file into blender, for `Object Types`, only select `Meshes`
Note: many of the meshes in the original scene use an animation base pose to position the object. Consider applying these transformations before exporting from blender.

Press 1, 2, or 3 for various camera locations. Press B for benchmark (see console for results).

```
Options:
  --minimal         disable bloom, AO, AA, shadows
  --random-materials
                    assign randomly generated materials to each unique mesh
                    (mesh instances also share materials)
  --texture-count   quantity of unique textures sets to randomly select from. (A
                    texture set being: base_color, roughness)
  --count           quantity of hotel 01 models
  --deferred        use deferred shading
  --no-frustum-culling
                    disable all frustum culling. Stresses queuing and batching
                    as all mesh material entities in the scene are always drawn.
  --no-automatic-batching
                    disable automatic batching. Skips batching resulting in
                    heavy stress on render pass draw command encoding.
  --no-view-occlusion-culling
                    disable gpu occlusion culling for the camera
  --no-shadow-occlusion-culling
                    disable gpu occlusion culling for the directional light
  --no-indirect-drawing
                    disable indirect drawing.
  --no-cpu-culling  disable CPU culling.
  --spin            spin the bistros and camera
  --hide-frame-time don't show frame time
  --help, help      display usage information
```
