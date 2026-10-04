---
title: Migrate and rename numeric input types to bevy_ui_widgets
pull_requests: [25868]
---

Migrated `NumberFormat`, `NumberInputValue` and `NumberInputRange` from `bevy_feathers` to `bevy_ui_widgets`. As part of this, renamed them to `NumericFormat`, `NumericValue` and `NumericRange`.

Update the imports as follows:

```diff
use bevy::{
-   feathers::{NumberFormat, NumberInputValue, NumberInputRange},
+   ui_widgets::{NumericFormat, NumericValue, NumericRange},
};
```
