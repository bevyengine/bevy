---
title: "`FontSize::Calc`"
authors: ["@ickshonpe"]
pull_requests: [26658]
---

`FontSize` has a new `Calc` variant which allows a font size to be the result of a calculation. 

`Calc`s are constructed using the `calc` method:

```
TextFont { font_size: FontSize::calc(|ctx| 5.0 + 0.02 * ctx.view.x) }
```
