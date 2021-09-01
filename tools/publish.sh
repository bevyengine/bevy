# if crate A depends on crate B, B must come before A in this list
crates=(
    bevy_utils
    bevy_macro_utils
    bevy_derive
    bevy_math
    bevy_tasks
    bevy_ecs/macros
    bevy_ecs
    bevy_app
    bevy_log
    bevy_dynamic_plugin
    bevy_reflect/bevy_reflect_derive
    bevy_reflect
    bevy_asset
    bevy_audio
    bevy_core
    bevy_diagnostic
    bevy_transform
    bevy_window
    bevy_render
    bevy_input
    bevy_gilrs
    bevy_pbr
    bevy_gltf
    bevy_scene
    bevy_sprite
    bevy_text
    bevy_ui
    bevy_winit
    bevy_wgpu
    bevy_internal
    bevy_dylib
)

cd crates
for crate in "${crates[@]}"
do
  echo "Publishing ${crate}"
  (cd "$crate"; cargo publish --no-verify)
  sleep 20
done

cd ..
cargo publish
