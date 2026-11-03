---
title: UI alignment `Default` variants renamed to `Normal`
pull_requests: []
---

The `Default` variants of `AlignItems`, `JustifyItems`, `AlignContent`, and `JustifyContent` have been renamed to `Normal` to match the `normal` alignment keyword from CSS. Any code using these enums should replace `Default` with `Normal`.

The `Default` trait implementations and `DEFAULT` constants for these enums are unchanged.
