---
title: Change filters container of `LogDiagnosticsState` to `HashSet`
authors: ["@hukasu"]
pull_requests: [19323]
---

`LogDiagnosticsState`'s filter container and the argument of
`LogDiagnosticPlugin::filtered` is now a `HashSet` rather than a `Vec`.
