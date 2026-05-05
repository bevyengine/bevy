---
title: "`audio` feature is now no longer implied by the `3d`, `2d`, or `ui` features"
pull_requests: [23126]
---

Our `default` features used to be

- 2d
- 3d
- ui

where each of these features enabled the `audio` feature among others.

Since Cargo doesn't allow selectively disabling features, if you wished to disable `bevy_audio`, either because you don't need audio or because you use an alternative such as bevy_seedling, you had a problem with that.
You needed to essentially enable all features enabled by the above *except* `audio`, leading to a big `features` soup. To avoid this,
`audio` is now no longer enabled by the above features, but instead enabled by default, bumping our default features to:

- 2d
- 3d
- ui
- audio

Now what does this mean for you?

- If you used all default features before, nothing changes for you.
- If you want to opt out of using `bevy_audio`, it is now as simple as disabling default features, and manually opting into `3d`, `2d`, and/or `ui`.
- If you already opted into non-default features and want to continue using `bevy_audio`, you will now have to add the `audio` feature.
