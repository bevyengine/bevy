---
title: ButtonInput for Key
authors: ["@kristoff3r"]
pull_requests: [19684]
---

Bevy now has a `ButtonInput<Key>` resource, similarly to the existing `ButtonInput<KeyCode>` resource.

The difference between `KeyCode` and `Key` is that the former refers to the
button location on a US keyboard independent of the actual layout in use, while
`Key` gives you the actual letter or symbol that was entered. In most cases you
still want to use `KeyCode`, but in some cases it makes more sense to use `Key`,
for example when using '+'/'-' to zoom.
