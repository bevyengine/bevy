---
title: "`SystemInput::Underlying` type"
pull_requests: []
---

Higher-order systems using a `StaticSystemInput<I>` parameter
were not usable as a system taking the inner `I` as input.
To fix this, a new associated type `Underlying` has been added to the `SystemInput` trait.

Manual implementations of `SystemInput` should add the new type,
which should normally be equal to `Self` with all lifetimes replaced by `'static`.
