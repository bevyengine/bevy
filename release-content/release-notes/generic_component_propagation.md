---
title: Generic component propagation
authors: ["@robtfm"]
pull_requests: [17575]
---

When working with large hierarchies of game objects, coordinating the state of the entire tree can be frustrating.
Bevy uses this pattern when working with transforms and visibility internally,
but users have had to reinvent the wheel every time they wanted to use similar patterns.

While this pain was most acute when working with [`RenderLayers`], this pattern is more broadly useful,
and has been exposed to end users in the form of the [`HierarchyPropagatePlugin`].
You might use this for synchronizing color and alpha values for "ghost" versions of previewed buildings,
ensuring that all of the parts of a model are on the same render layer,
or propagating font styles.

This plugin has three generics:

- `C: Component`: the type of component that should be propagated
- `F: QueryFilter=()`: if set, only entities which match this filter will be affected by propagation
- `R: Relationship = ChildOf`: the type of tree-like relationship to propagate down

Each copy of this plugin will propagate components of type `C` down the hierarchy, along all entities which match the
query filter of type `F`.
With this plugin enabled for `C`, you can add a [`Propagate<C>`] component to add new components to all children,
add a [`PropagateStop<C>`] component to stop propagation, or even use [`PropagateOver<C>`] to skip this entity during propagation.

This is a very general tool: please let us know what you're using it for and we can continue to add examples to the docs!

[`RenderLayers`]: https://dev-docs.bevy.org/bevy/camera/visibility/struct.RenderLayers.html
[`HierarchyPropagatePlugin`]: https://dev-docs.bevy.org/bevy/app/struct.HierarchyPropagatePlugin.html
[`Propagate<C>`]: https://dev-docs.bevy.org/bevy/app/struct.Propagate.html
[`PropagateStop<C>`]: https://dev-docs.bevy.org/bevy/app/struct.PropagateStop.html
[`PropagateOver<C>`]: https://dev-docs.bevy.org/bevy/app/struct.PropagateOver.html
