---
title: Per-column change ticks
authors: ["@pcwalton", "@SkiFire13"]
pull_requests: [25157, 25429]
---

A new summary change tick is now stored per-column, representing the last time any component in that column was changed. This allows skipping the whole column in case no component was changed, as opposed to going through each component checking them individually.
