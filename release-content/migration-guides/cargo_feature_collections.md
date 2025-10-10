---
title: Cargo Feature Collections
pull_requests: [21472]
---

Bevy now has high-level cargo feature collections (ex: `2d`, `3d`, `ui`) and mid-level feature collections (ex: `2d_api`, `3d_api`, `default_app`, `default_platform`). This isn't technically a breaking change, but if you were previously disabling Bevy's default features and manually enabling each specific cargo feature you wanted, we _highly_ recommend switching to using the higher level feature collections wherever possible. This will make it much easier to define the functionality you want, and it will reduce the burden of keeping your list of features up to date across Bevy releases.

See the Cargo Feature Collections pull request for a full list of options.
