# if crate A depends on crate B, B must come before A in this list
crates=(
    bevy_utils
    bevy_ptr
    bevy_macro_utils
    bevy_derive
    bevy_math
    bevy_color
    bevy_tasks
    bevy_reflect/derive
    bevy_reflect
    bevy_ecs/macros
    bevy_ecs
    bevy_state/macros
    bevy_state
    bevy_app
    bevy_time
    bevy_log
    bevy_asset/macros
    bevy_asset
    bevy_audio
    bevy_diagnostic
    bevy_hierarchy
    bevy_transform
    bevy_window
    bevy_encase_derive
    bevy_render/macros
    bevy_mikktspace
    bevy_image
    bevy_mesh
    bevy_render
    bevy_core_pipeline
    bevy_input
    bevy_gilrs
    bevy_animation
    bevy_pbr
    bevy_gltf
    bevy_remote
    bevy_scene
    bevy_picking
    bevy_sprite
    bevy_gizmos/macros
    bevy_gizmos
    bevy_text
    bevy_a11y
    bevy_input_focus
    bevy_ui
    bevy_winit
    bevy_dev_tools
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
  cp ../LICENSE-MIT "$crate"
  cp ../LICENSE-APACHE "$crate"
  pushd "$crate"
  git add LICENSE-MIT LICENSE-APACHE
  cargo publish --no-verify --allow-dirty
  popd
  sleep 20
done

popd

echo "Publishing root crate"
cargo publish --allow-dirty

echo "Cleaning local state"
git reset HEAD --hard
