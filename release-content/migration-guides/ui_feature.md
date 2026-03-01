---
title: "`ui` feature is now no longer implied by the `3d` or `2d` features"
pull_requests: [23180]
---

Swapping the UI framework for your Bevy project is a common form of customization.
We think that users should be able to do this easily, without having to give up the ease of use (and updates!)
that come with our top-level feature collections.

To achieve this, the `ui` feature collection is now no longer implied by the `3d` or `2d` feature collection.

To migrate:

- If you used all default features before, nothing changes for you.
- If you want to opt out of using `bevy_ui`, it is now as simple as disabling default features, and manually opting into `3d` or `2d` (and optionally `audio`).
- If you already opted into non-default features and want to continue using `bevy_ui`, you will now have to add the `ui` feature.
