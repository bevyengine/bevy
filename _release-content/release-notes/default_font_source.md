---
title: Default font source
authors: ["@ickdhonpe"]
pull_requests: [25847]
---

`DefaultFontSource` is a new resource that newtypes `FontSource` and can be used to set a global default font.
`FontSource` has a new variant `Default`. During font resolution if `FontSource::Default` is found, the `FontSource` from `DefaultFontSource` is used instead.
`DefaultFontSource(FontSource::Default)` is mapped to `FontSource::Handle(Handle::default())`.
