---
title: "Feathers menu improvements"
authors: ["@viridia"]
pull_requests: [24784]
---

In addition to `FeathersMenu`, there is now `FeathersLazyMenu`. While the former expects a
pre-spawned popup entity which is hidden, the latter dynamically spawns the popup when the menu
is opened, and and despawns it when closed. This is the recommended approach for menu popups
whose list of menu items is long, expensive to keep around, or dynamically-generated.

In addition to `FeathersMenuButton`, there is now also `FeathersMenuToolButton` which works the
same as a regular menu button but which has the tool button form factor.
