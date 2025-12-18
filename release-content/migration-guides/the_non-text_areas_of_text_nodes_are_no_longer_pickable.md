---
title: "The non-text areas of UI `Text` nodes are no longer pickable"
pull_requests: [22047]
---

Only the sections of `Text` node's containing text are pickable now, the non-text areas of the node do not register pointer hits.
To replicate Bevy 0.17's picking behavior, use an intermediate parent node to intercept the pointer hits.
