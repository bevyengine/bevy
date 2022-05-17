# if crate A depends on crate B, B must come before A in this list
crates=(
    bevy_utils
    bevy_ptr
    bevy_macro_utils
    bevy_derive
    bevy_math
    bevy_tasks
    bevy_reflect/bevy_reflect_derive
    bevy_reflect
    bevy_ecs/macros
    bevy_ecs
    bevy_app
    bevy_log
    bevy_dynamic_plugin
    bevy_asset
    bevy_audio
    bevy_core
    bevy_diagnostic
    bevy_hierarchy
    bevy_transform
    bevy_window
    bevy_crevice/bevy-crevice-derive
    bevy_crevice
    bevy_render
    bevy_core_pipeline
    bevy_input
    bevy_gilrs
    bevy_animation
    bevy_pbr
    bevy_gltf
    bevy_scene
    bevy_sprite
    bevy_text
    bevy_ui
    bevy_winit
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
