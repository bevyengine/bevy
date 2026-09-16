---
title: World summaries
authors: ["@jbuehler23", "@alice-i-cecile"]
pull_requests: [25818]
---

`bevy_dev_tools` can now produce a statistical summary of a `World`: entity and archetype counts, resource counts, and a per-archetype breakdown of components, entity counts, and memory size per entity.

Call `World::summarize` with a `SummarySettings` to get a `WorldSummary`, or `Commands::summarize` to log one. The settings control component-name resolution, whether empty archetypes are included, and how many archetype rows the `Display` output prints.
