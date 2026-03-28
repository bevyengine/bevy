---
title: Bevy Remote Protocol, schedules and `RenderApp`
authors: ["@zeophlite"]
pull_requests: [23446, 23447, 23452]
---

BRP now has methods for schedule introspection:

- `schedule.list` lists all schedules
- `schedule.graph` gives the system sets and their dependencies in a schedule

Check the PR's for details on these methods.

BRP now also runs in the Render World.  This has all the same methods, and runs on port `15703` by default.
