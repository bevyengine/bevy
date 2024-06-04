use bevy_math::UVec2;

use crate::{ClusterConfig, Clusters};

fn test_cluster_tiling(config: ClusterConfig, screen_size: UVec2) -> Clusters {
    let dims = config.dimensions_for_screen_size(screen_size);

    // note: near & far do not affect tiling
    let mut clusters = Clusters::default();
    clusters.update(screen_size, dims);

    // check we cover the screen
    assert!(clusters.tile_size.x * clusters.dimensions.x >= screen_size.x);
    assert!(clusters.tile_size.y * clusters.dimensions.y >= screen_size.y);
    // check a smaller number of clusters would not cover the screen
    assert!(clusters.tile_size.x * (clusters.dimensions.x - 1) < screen_size.x);
    assert!(clusters.tile_size.y * (clusters.dimensions.y - 1) < screen_size.y);
    // check a smaller tile size would not cover the screen
    assert!((clusters.tile_size.x - 1) * clusters.dimensions.x < screen_size.x);
    assert!((clusters.tile_size.y - 1) * clusters.dimensions.y < screen_size.y);
    // check we don't have more clusters than pixels
    assert!(clusters.dimensions.x <= screen_size.x);
    assert!(clusters.dimensions.y <= screen_size.y);

    clusters
}

#[test]
// check tiling for small screen sizes
fn test_default_cluster_setup_small_screensizes() {
    for x in 1..100 {
        for y in 1..100 {
            let screen_size = UVec2::new(x, y);
            let clusters = test_cluster_tiling(ClusterConfig::default(), screen_size);
            assert!(clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z <= 4096);
        }
    }
}

#[test]
// check tiling for long thin screen sizes
fn test_default_cluster_setup_small_x() {
    for x in 1..10 {
        for y in 1..5000 {
            let screen_size = UVec2::new(x, y);
            let clusters = test_cluster_tiling(ClusterConfig::default(), screen_size);
            assert!(clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z <= 4096);

            let screen_size = UVec2::new(y, x);
            let clusters = test_cluster_tiling(ClusterConfig::default(), screen_size);
            assert!(clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z <= 4096);
        }
    }
}
