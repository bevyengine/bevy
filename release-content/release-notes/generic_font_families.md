---
title: "Generic Font Families"
authors: ["@ickshonpe"]
pull_requests: [22396]
---

Support for generic font families has been added through new `FontSource` variants: `Serif`, `SansSerif`, `Cursive`, `Fantasy`, and `Monospace`.

The `CosmicFontSystem` resource can be used to update the font family associated with each generic font variant:

```rust
let mut font_system = world.resource_mut::<CosmicFontSystem>();
font_system.db_mut().set_serif_family("Allegro");
font_system.db_mut().set_sans_serif_family("Encode Sans");
font_system.db_mut().set_cursive_family("Cedarville Cursive");
font_system.db_mut().set_fantasy_family("Argusho");
font_system.db_mut().set_monospace_family("Lucida Console");

// Use `get_family` to retrieve the family name associated with a `FontSource`'.
font_system.get_family(FontSource::Serif);
assert_eq!(family_name, "Allegro");
```
