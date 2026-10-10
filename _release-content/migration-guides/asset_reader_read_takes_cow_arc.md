---
title: AssetReader::read and AssetReader::read_meta now take a CowArc Path.
pull_requests: [26092]
---

Previously `AssetReader::read` and `AssetReader::read_meta` take a `Path` borrow. This limited the
use of these methods since they `Path` had to **outlive** the reader.

Now, these methods both take `CowArc<'_, Path>`, allowing you to **move an owned `Path`** into these
methods, so the resulting readers are only tied to the lifetime of the asset source itself.

Just change your `AssetReader` implementations like so:

```rust
// Before
impl AssetReader for MyReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        unimplemented!()
    }

    async fn read_meta<'a>(
      &'a self,
      path: &'a Path,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        unimplemented!()
    }

    // Everything else is the same...
}

// After

// You may need to import CowArc. We've provided a type-alias for it!
use bevy::asset::CowArc;

impl AssetReader for MyReader {
    async fn read<'a>(
        &'a self,
        path: CowArc<'a, Path>,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        unimplemented!()
    }

    async fn read_meta<'a>(
      &'a self,
      path: CowArc<'a, Path>,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        unimplemented!()
    }

    // Everything else is the same...
}
```
