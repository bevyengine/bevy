Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering

Each frame, this will be reset to `false` during [`VisibilityPropagate`] systems in [`PostUpdate`].
Later in the frame, systems in [`CheckVisibility`] will mark any visible entities using [`ViewVisibility::set`](`bevy_render::view::visibility::ViewVisibility::set`).
Because of this, values of this type will be marked as changed every frame, even when they do not change.

If you wish to add custom visibility system that sets this value, make sure you add it to the [`CheckVisibility`] set.

[`VisibilityPropagate`]: bevy_render::view::visibility::VisibilitySystems::VisibilityPropagate
[`CheckVisibility`]: bevy_render::view::visibility::VisibilitySystems::CheckVisibility
[`PostUpdate`]: bevy_app::PostUpdate