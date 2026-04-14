---
title: "Feathers widgets moving to BSN"
pull_requests: [23804]
---

Going forward, BSN will be the primary means to create Feathers widgets. The old spawning
functions have been renamed (`button` is now `button_bundle`), and will be removed in a future
release.

Some of the BSN widgets are slightly different than before:

- `button` no longer automatically includes `flex_grow`. This was originally added due to the
  difficulty of overriding node styles when spawning, but in BSN that's no longer a problem.
- `button`, `checkbox` and `radio` now accept a `caption` parameter which lets you specify
  the label directly instead of appending them via `Children`.
