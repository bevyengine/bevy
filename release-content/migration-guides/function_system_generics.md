---
title: "FunctionSystem Generics"
authors: ["@ecoskey"]
pull_requests: [21917]
---

`FunctionSystem` now has a new generic parameter: `In`.

Old: `FunctionSystem<Marker, Out, F>`
New: `FunctionSystem<Marker, In, Out, F>`

Additionally, there's an extra bound on the `System` and `IntoSystem` impls
related to `FunctionSystem`:

`<F as SystemParamFunction>::In: FromInput<In>`

This enabled systems to take as input any *compatible* type, in addition to the
exact one specified by the system function. This shouldn't impact users at all
since it only adds functionality, but users writing heavily generic code may
want to add a similar bound. See `function_system.rs` to see how it works in
practice.
