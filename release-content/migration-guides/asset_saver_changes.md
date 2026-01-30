---
title: SavedAsset now contains two lifetimes.
pull_requests: []
---

`SavedAsset` now holds two lifetimes instead of one. This is primarily used in the context of
`AssetSaver`. So previously, users may have:

```rust
impl AssetSaver for MySaver {
    type Asset = MyAsset;
    type Settings = ();
    type OutputLoader = MyLoader;
    type Error = std::io::Error;

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, Self::Asset>,
        settings: &Self::Settings,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}
```

Now with the extra `SavedAsset` lifetime:

```rust
impl AssetSaver for MySaver {
    type Asset = MyAsset;
    type Settings = ();
    type OutputLoader = MyLoader;
    type Error = std::io::Error;

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, '_, Self::Asset>,
        settings: &Self::Settings,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}
```

In practice, this should not have an impact on usages.
