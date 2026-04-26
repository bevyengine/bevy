---
title: AssetPath now stores a str instead of a Path.
pull_requests: [23992]
---

In previous versions of Bevy, `AssetPath` internally stored `CowArc<'_, Path>`. This awkwardly took
our platform-agnostic asset system and coupled it to the path representation of the target platform.
This could result in different behavior of asset paths depending on what platform you authored
assets on versus what platform you compile for.

Now, `AssetPath` internally stores `CowArc<'_, str>`. This means the handling of paths is entirely
up to us (and users). The advantage is now the platform is **only** relevant when interacting with
the non-platform-agnostic asset sources. This also allows asset sources to be more flexible in their
implementations, allowing the path to be a more free-form descriptor.

Of course, handling raw strings is not ideal, so we've created utilities for common path operations.
Below is a list of some common `Path` operations and their equivalent:

- `path.extension()` -> `path_file_extension(path)`
- `path.file_name()` -> `path_basename(path)`
- `path.parent()` -> `path_parent(path)`
- `path.is_absolute()` -> `is_absolute_path(path)`
- `path.ancestors()` -> `path_ancestors(path)`
- `path.join(other)` -> `join_paths(path, other)`
- `path.components()` *or* `path.iter()` -> `path_components(path)`

When creating asset paths, also consider using `clean_path` to convert raw paths (which may contain
platform-specific features) into platform-agnostic paths.

In addition, `AssetPath::from_path` and `AssetPath::from_path_buf` have been replaced by
`AssetPath::from_str_path` and `AssetPath::from_string_path` respectively.
