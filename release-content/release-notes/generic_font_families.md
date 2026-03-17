---
title: "Generic Font Families"
authors: ["@ickshonpe"]
pull_requests: [22396, 22879]
---

Bevy now supports generic font families, allowing font faces to be selected using broadly defined categories (such as `FontSource::Serif` or `FontSource::Monospace`) without naming a specific font family.

The `FontCx` resource can be used to update the font family associated with each generic font variant:

```rust
fn font_families(mut font_system: ResMut<FontCx>) {
  font_system.set_serif_family("Allegro");
  font_system.set_sans_serif_family("Encode Sans");
  font_system.set_cursive_family("Cedarville Cursive");
  font_system.set_fantasy_family("Argusho");
  font_system.set_monospace_family("Lucida Console");

  // `FontCx::get_family` can be used to look up the family associated with a `FontSource`
  let family_name = font_system.get_family(&FontSource::Serif).unwrap();
  assert_eq!(family_name.as_str(), "Allegro");
}
```
