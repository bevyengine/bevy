---
title: "Text Background Colors"
authors: ["@ickshonpe"]
pull_requests: [18892, 20464]
---

TODO: add showcase image(s)

Text in Bevy now supports background colors. Insert the `TextBackgroundColor` component on a UI `Text` or `TextSpan` entity to set a background color for its text section. `TextBackgroundColor` provides the ability to set the color of _each_ "text span", whereas the standard `BackgroundColor` applies to _all_ spans in a `Text` node, and also includes space taken up by padding.

`TextBackgroundColor` also works with `Text2d`: perfect for worldspace tooltips!
