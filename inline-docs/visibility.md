Describes the visibility properties of the node.
A user indication of whether an entity is visible. Propagates down the entity hierarchy.

If an entity is hidden in this way, all [`Children`](`bevy_hierarchy::Children`) (and all of their children and so on) who are set to [`Visibility::Inherited`](`bevy_render::view::visibility::Visibility::Inherited`) will also be hidden.

This is done by the `visibility_propagate_system` which uses the entity hierarchy and [`Visibility`](`bevy_render::view::visibility::Visibility`) to set the values of each entity's [`InheritedVisibility`](`bevy_render::view::visibility::InheritedVisibility`) component.