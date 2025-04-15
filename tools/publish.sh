crates=(
    bevy_macro_utils
    bevy_derive
    bevy_ecs/macros
    bevy_platform
    bevy_ptr
    bevy_reflect/derive
    bevy_utils
    bevy_reflect
    bevy_tasks
    bevy_ecs
    bevy_app
    bevy_math
    bevy_input
    bevy_window
    bevy_input_focus
    bevy_a11y
    bevy_asset/macros
    bevy_log
    bevy_asset
    bevy_color
    bevy_time
    bevy_diagnostic
    bevy_encase_derive
    bevy_image
    bevy_mikktspace
    bevy_transform
    bevy_mesh
    bevy_render/macros
    bevy_render
    bevy_animation
    bevy_audio
    bevy_core_pipeline
    bevy_picking
    bevy_state/macros
    bevy_state
    bevy_sprite
    bevy_text
    bevy_ui
    bevy_dev_tools
    bevy_gilrs
    bevy_gizmos/macros
    bevy_pbr
    bevy_gizmos
    bevy_scene
    bevy_gltf
    bevy_remote
    bevy_winit
    bevy_internal
    bevy_dylib
)

if [ -n "$(git status --porcelain)" ]; then
    echo "You have local changes!"
    exit 1
fi

pushd crates

for crate in "${crates[@]}"
do
  echo "Publishing ${crate}"
  pushd "$crate"
  cargo publish
  popd
done

popd

echo "Publishing root crate"
cargo publish
