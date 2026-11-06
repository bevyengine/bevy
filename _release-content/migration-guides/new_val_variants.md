---
title: "New `Val` variants"
pull_requests: [26416]
---

Additional variants have been added to `Val`: `MinContent`, `MaxContent`, `FitContent`, `Stretch`, and `Content`. 
They are only supported by `Node`'s `width`, `height` and `flex_basis` fields, otherwise they are interpreted as `Auto`.
Any exhaustive matches on `Val` will need to be updated.
