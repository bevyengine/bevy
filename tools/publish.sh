# if crate A depends on crate B, B must come before A in this list
crates=(
    bevy_utils
    bevy_derive
    bevy_math
    bevy_tasks
    bevy_ecs/hecs/macros
    bevy_ecs/hecs
    bevy_ecs
    bevy_app
    bevy_dynamic_plugin
    bevy_property/bevy_property_derive
    bevy_property
    bevy_type_registry
    bevy_asset
    bevy_audio
    bevy_core
    bevy_diagnostic
    bevy_transform
    bevy_window
    bevy_render
    bevy_gltf
    bevy_input
    bevy_gilrs
    bevy_pbr
    bevy_scene
    bevy_sprite
    bevy_text
    bevy_ui
    bevy_winit
    bevy_wgpu
)

cd crates
for crate in "${crates[@]}"
do
  echo "Publishing ${crate}"
  (cd $crate; cargo publish)
  sleep 15
done

cd ..
cargo publish