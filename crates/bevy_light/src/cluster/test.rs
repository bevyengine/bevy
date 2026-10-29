use bevy_math::UVec2;

use super::{
    assign::{calculate_cluster_factors, cluster_far_z},
    ClusterConfig, Clusters,
};

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

fn assert_finite_cluster_factors(near: f32, far: f32, is_orthographic: bool) {
    for z_slices in [1.0, 24.0] {
        let far = cluster_far_z(near, far, is_orthographic);
        assert!(far > near, "near {near}, far {far}");
        let factors = calculate_cluster_factors(near, far, z_slices, is_orthographic);
        assert!(
            factors.is_finite(),
            "near {near}, far {far}, z_slices {z_slices}, is_orthographic {is_orthographic}: {factors}"
        );
    }
}

#[test]
// no clusterable object reaches past the first slice
fn test_cluster_factors_finite_when_far_less_than_near() {
    for near in [0.1, 5.0, 1000.0] {
        for far in [near, near * 0.5, 0.0, f32::NAN] {
            assert_finite_cluster_factors(near, far, false);
        }
    }
    for near in [-1000.0, -0.5, 0.0, 0.1, 5.0] {
        for far in [near, near - 1.0, f32::NAN] {
            assert_finite_cluster_factors(near, far, true);
        }
    }
}
