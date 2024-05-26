Inherited visibility of an entity.

This will not be accurate until [`VisibilityPropagate`](`bevy_render::view::visibility::VisibilitySystems`) runs in the [`PostUpdate`](`bevy_app::PostUpdate`) schedule.

If this is false, then [`ViewVisibility`](`bevy_render::view::visibility::ViewVisibility`) should also be false.