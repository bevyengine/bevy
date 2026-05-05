---
title: "`bevy_shader` cleanups"
pull_requests: [22774]
---

`ShaderReflectError` has been deleted, as it was unused.

`ShaderCache::new` now accepts a `RenderDevice`, and `ShaderCache::get` does not. This is to reflect the fact that a `ShaderCache` must only be used with one `RenderDevice` for it to be valid.

The `set_import_path`, `with_import_path`, `import_path`, and `imports` methods on `Shader` have been removed. Just access the fields directly, these were superfluous getter methods.
