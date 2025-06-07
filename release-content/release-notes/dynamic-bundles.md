---
title: Option and dynamic bundles
authors: ["@SkiFire13"]
pull_requests: [19491]
---

Until now bundles have always represented a static set of components, meaning you could not create a `Bundle` that sometimes inserts a component and sometimes doesn't, or a totally dynamic bundle like you would expect from a `Box<dyn Bundle>`.

In Bevy 0.17 the `Bundle` trait has been reworked to support these usecases, in particular:

- `Option<B: Bundle` now implements `Bundle`
- `Bundle` is now a dyn-compatible trait and `Box<dyn Bundle>` implements `Bundle`
- `Vec<Box<dyn Bundle>>` also implements `Bundle`

TODO: showcase
