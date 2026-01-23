---
title: Labeled assets are now called subassets.
pull_requests: []
---

Previously, we had two terms for assets that "live inside" another - either "subasset" or "labeled
asset". Our docs would sometimes call them one or the other (sometimes both!).

We've now replaced all instances of "labeled asset" with "subasset". To this end, the following
types/functions have been renamed:

- `LoadedAsset::get_labeled` -> `LoadedAsset::get_subasset`
- `LoadedAsset::iter_labels` -> `LoadedAsset::iter_subasset_names`
- `ErasedLoadedAsset::get_labeled` -> `ErasedLoadedAsset::get_subasset`
- `ErasedLoadedAsset::iter_labels` -> `ErasedLoadedAsset::iter_subasset_names`
- `LoadContext::begin_labeled_asset` -> `LoadContext::begin_subasset`
- `LoadContext::labeled_asset_scope` -> `LoadContext::subasset_scope`
- `LoadContext::add_labeled_asset` -> `LoadContext::add_subasset`
- `LoadContext::add_loaded_labeled_asset` -> `LoadContext::add_loaded_subasset`
- `LoadContext::has_labeled_asset` -> `LoadContext::has_subasset`
- `LoadContext::get_label_handle` -> `LoadContext::get_subasset_handle`
- `ParseAssetPathError::InvalidLabelSyntax` -> `ParseAssetPathError::InvalidSubassetSyntax`
- `ParseAssetPathError::MissingSubassetName` -> `ParseAssetPathError::MissingSubassetName`
- `AssetPath::label` -> `AssetPath::subasset_name`
- `AssetPath::label_cow` -> `AssetPath::subasset_name_cow`
- `AssetPath::without_label` -> `AssetPath::without_subasset_name`
- `AssetPath::remove_label` -> `AssetPath::remove_subasset_name`
- `AssetPath::take_label` -> `AssetPath::take_subasset_name`
- `AssetPath::with_label` -> `AssetPath::with_subasset_name`
- `SavedAsset::get_labeled` -> `SavedAsset::get_subasset`
- `SavedAsset::get_erased_labeled` -> `SavedAsset::get_erased_subasset`
- `SavedAsset::iter_labels` -> `SavedAsset::iter_subasset_names`
- `TransformedAsset::take_labeled_assets` -> `TransformedAsset::take_subassets`
- `TransformedAsset::get_labeled` -> `TransformedAsset::get_subasset`
- `TransformedAsset::get_erased_labeled` -> `TransformedAsset::get_erased_subasset`
- `TransformedAsset::insert_labeled` -> `TransformedAsset::insert_subasset`
- `TransformedAsset::iter_labels` -> `TransformedAsset::iter_subasset_names`
- `TransformedSubAsset::get_labeled` -> `TransformedSubAsset::get_subasset`
- `TransformedSubAsset::get_erased_labeled` -> `TransformedSubAsset::get_erased_subasset`
- `TransformedSubAsset::insert_labeled` -> `TransformedSubAsset::insert_subasset`
- `TransformedSubAsset::iter_labels` -> `TransformedSubAsset::iter_subasset_names`
- `GltfAssetLabel` -> `GltfSubassetName`
- `GltfMesh::asset_label` -> `GltfMesh::subasset_name`
- `GltfNode::asset_label` -> `GltfNode::subasset_name`
- `GltfPrimitive::asset_label` -> `GltfPrimitive::subasset_name`
- `GltfSkin::asset_label` -> `GltfSkin::subasset_name`
