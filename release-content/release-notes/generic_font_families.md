---
title: "Generic Font Families"
authors: ["@ickshonpe"]
pull_requests: [22396]
---

Support for generic font families has been added through new `FontSource` variants: `Serif`, `SansSerif`, `Cursive`, `Fantasy`, and `Monospace`.

The `CosmicFontSystem` resource can be used to update the font family associated with each generic font variant:

```rust
let mut font_system = CosmicFontSystem::default();
let mut font_database = font_system.db_mut();
font_database.set_serif_family("Allegro");
font_database.set_sans_serif_family("Encode Sans");
font_database.set_cursive_family("Cedarville Cursive");
font_database.set_fantasy_family("Argusho");
font_database.set_monospace_family("Lucida Console");

// `CosmicFontSystem::get_family` can be used to look the family associated with a `FontSource`
let family_name = font_system.get_family(&FontSource::Serif).unwrap();
assert_eq!(family_name.as_str(), "Allegro");
```
