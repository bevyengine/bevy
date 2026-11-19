---
title: `FontSize::default()` is now `Rem(1.)`
pull_requests: [26611]
---

The default for `FontSize` has been changed from `Px(20.)` to `Rem(1.)`.

Since the default rem size is 20px, default sized text still renders at 20px.
If you change the `RemSize`, then to keep that text at 20px you'll need to set its font size to `FontSize::Px(20.)` explicitly.
