---
title: Contact Shadows
pull_requests: [ 22382 ]
---

The `shadows_enabled` field on `PointLight`, `DirectionalLight`, and `SpotLight` has changed to `shadow_maps_enabled`.

This was changed because these lights now support contact shadows, and have a `contact_shadows_enabled` field. The old `shadows_enabled` field only configures shadow maps, making the old name misleading.