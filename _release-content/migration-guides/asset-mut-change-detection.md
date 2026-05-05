---
title: "Avoiding unnecessary `AssetEvent::Modified` events that lead to rendering performance costs"
pull_requests: [22460]
---

`Assets::get_mut` will now return `AssetMut<A: Asset>` instead of `&mut Asset`.
Similar to `Mut`/`ResMut`, new implementation will trigger `AssetEvent::Modified`
event only when the asset is actually mutated.

In some cases (like materials), triggering `AssetEvent::Modified` event might lead to
measurable performance costs. To avoid this, it is now possible to check if the `Asset`
will change before mutating it:

```rust
fn update(
    query: Query<MeshMaterial3d<StandardMaterial>>,
    materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for material_handle in query.iter_mut() {
        // material variable now needs to be marked as mut
        let Some(mut material) = materials.get_mut(material_handle) else {
            continue;
        };
        
        let new_color = compute_new_color(&time);
        if material.base_color != new_color {
            // material will be marked as changed and extracted down the line
            // only if the color has actually changed
            material.base_color = new_color;
        }
    }
}
```
