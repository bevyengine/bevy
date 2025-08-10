---
title: `ScrollPosition` now uses logical pixel units and is no longer overwritten during layout updates
pull_requests: [20093]
---
`ScrollPosition` is no longer overwritten during layout updates. Instead the computed scroll position is stored in the new `scroll_position` field on `ComputedNode`.
