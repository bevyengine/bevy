---
title: `App` startup schedules
pull_requests: [20407]
---

In previous versions of Bevy, the `Startup` schedule (and its friends) were really a part of
updating the app. Every frame, we just check "have we run the startup schedule", and if not (which
is only true for the first frame), we run the startup schedules. This worked totally fine for the
main world.

However, it causes a problem for the renderer (and any other sub-apps users create). The renderer is
not able to initialize its state until after the first game frame is simulated. This is a problem if
this initialization takes some time (e.g., there are some async tasks we have to wait on). We can
only start those tasks after the first game frame, at which point we may need to block.

In Bevy 0.17, sub-apps now have an explicit `startup_schedule`. All startup schedules will run
**before** the update schedule is run (allowing sub-apps to be initialized before we start
simulating the game frame). In particular, the `RenderStartup` schedule is now the startup schedule
for the render app!
