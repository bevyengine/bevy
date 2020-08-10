crates=(
    bevy_app
    bevy_asset
    bevy_audio
    bevy_core
    bevy_derive
    bevy_diagnostic
    bevy_ecs
    bevy_ecs/hecs
    bevy_ecs/hecs/macros
    bevy_gltf
    bevy_input
    bevy_math
    bevy_pbr
    bevy_property
    bevy_render
    bevy_scene
    bevy_sprite
    bevy_text
    bevy_transform
    bevy_type_registry
    bevy_ui
    bevy_wgpu
    bevy_window
    bevy_winit
)

cd crates
for crate in "${crates[@]}"
do
  echo "Publishing ${crate}"
  (cd $crate; cargo build)
done