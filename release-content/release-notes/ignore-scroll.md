---
title: Support for Ui nodes that ignore parent scroll position.
authors: ["@PPakalns"]
pull_requests: [21648]
---

Adds the `IgnoreScroll` component, which controls whether a UI element ignores its parent’s `ScrollPosition` along specific axes.

This can be used to achieve basic sticky row and column headers in scrollable UI layouts. See `scroll` example.
