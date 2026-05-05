---
title: SavedAsset now contains two lifetimes, and AssetSaver now takes an AssetPath.
pull_requests: []
---

`SavedAsset` now holds two lifetimes instead of one. This is primarily used in the context of
`AssetSaver`. `AssetSaver` also now takes an `AssetPath`. So previously, users may have:

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

Now with the extra `SavedAsset` lifetime, and the extra `AssetPath`:

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
        asset_path: AssetPath<'_>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}
```

In practice, this should not have an impact on usages.
