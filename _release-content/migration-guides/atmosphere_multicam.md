---
title: "Atmosphere now supports multiple cameras"
pull_requests: [23113]
---

Atmosphere now works correctly with multiple cameras. No action is required for most users.

`init_atmosphere_buffer` has been removed, and `AtmosphereBuffer` has been changed from a `Resource` to a `Component` attached to each camera entity.

If you were directly accessing `AtmosphereBuffer` as a resource in a render world system, you'll need to query for it as a component on camera entities instead.
