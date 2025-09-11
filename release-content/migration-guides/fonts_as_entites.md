---
title: Fonts as entities
pull_requests: [20966]
---

Fonts as entities. 

Major changes:
* What was the `Font` asset has been renamed to `FontFace`.
* All the fields from `TextFont` are removed, it now newtypes an entity instead.
* `LineHeight` is now a component.
* New component `Font` with `face`, `size` and `smoothing` fields.
* Fonts are represented by entities with a `Font` component.
* The `DefaultFont` marker component can be added to a `Font` entity to make it the default font. 
* Enabling the "default_font" feature spawns a font entity with the default font and the `DefaultFault` marker component.