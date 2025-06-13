---
title: Observers now have name
pull_requests: [19611]
---

Observer will now spawn with a `Name` component when spawning with `add_observer` or `observe`, this might
cause unwanted triggers of `On<OnAdd, Name>`.
