---
title: "`Text2d` Shadows"
authors: ["@ickshonpe"]
pull_requests: [20463]
---

TODO: add showcase image(s)

`Text2d` is a simple worldspace text API: great for damage numbers and simple labels. Unlike `Text`, its UI sibling, it didn't support drop shadows, so in **Bevy 0.17** we've added dropshadow support to `Text2d`. Add the `Text2dShadow` component to a `Text2d` entity to draw a shadow effect beneath its text.
