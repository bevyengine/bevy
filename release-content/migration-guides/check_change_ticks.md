---
title: `CheckChangeTicks` parameter in `System::check_change_tick`
pull_requests: [19274, 19600]
---

`System::check_change_tick` took a `Tick` parameter to update internal ticks. This is needed to keep queried components filtered by their change tick reliably not be matched if their last change or add and the system's last run was very long ago. This is also needed for similar methods involving the system's ticks for the same reason.

This parameter is now a `CheckChangeTicks` type that can be passed to the now-public `Tick::check_tick` in case you maintain these yourself in manual `System` implementations. This type is also an event that can be observed.
