---
title: "Ornamented `Text2d`"
authors: ["@ickshonpe"]
pull_requests: [20463, 20464]
---

TODO: add showcase image(s)

`Text2d` is a simple worldspace text API: great for damage numbers and simple labels.
It deserves a few bells and whistles though, so it can keep up with its UI brother, `Text`.

You can now set the background color of `Text2d` with the `TextBackgroundColor` component.
Add a `TextBackgroundColor` to `Text2d` entity or its child `TextSection` entities to draw a background color for that section of text.

We've also added dropshadow support for `Text2d`. Add the `Text2dShadow` component to a `Text2d` entity to draw a shadow effect beneath its text.
