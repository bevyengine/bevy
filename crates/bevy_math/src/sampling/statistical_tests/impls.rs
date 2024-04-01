use super::traits::{BinDistribution, Binned, WithBinDistributions};
use crate::{
    primitives::*,
    sampling::{BoundaryOf, InteriorOf},
    Vec2, Vec3, Vec3Swizzles,
};
use std::f32::consts::PI;

//------------------//
// Helper Functions //
//------------------//

/// Given a value `v` between `a` and `b`, return the value `t` for which `v == a.lerp(b, t)`.
#[inline]
fn inverse_lerp(v: f32, a: f32, b: f32) -> f32 {
    (v - a) / (b - a)
}

/// Given a `value` and the `lower` and `upper` bounds of an interval partitioned evenly into `bins`
/// bins, return the index of the bin that `value` falls into.
///
/// Returns None when `value` is outside the interval.
fn bin_from_range(value: f32, lower: f32, upper: f32, bins: usize) -> Option<usize> {
    let t = inverse_lerp(value, lower, upper);
    if !(0. ..=1.).contains(&t) {
        None
    } else {
        let multiplier: f32 = bins as f32;
        Some((t * multiplier).floor() as usize)
    }
}

/// Given the `start` and `end` of an interval to be partitioned into `segments` segments, return a
/// vector of length `segments` enumerating the breakpoints of those intervals. The `end` of the
/// interval is included, but the `start` is not.
fn partition_range(start: f32, end: f32, segments: usize) -> Vec<f32> {
    let mut output = Vec::with_capacity(segments);
    let step = (end - start) / (segments as f32);
    for i in 0..segments {
        output.push(start + step * ((i + 1) as f32));
    }
    output
}

//--------------------------//
// Concrete Implementations //
//--------------------------//

/// A discretized distribution for the interior of a [`Circle`].
#[derive(Clone, Copy)]
pub struct CircleInteriorBins {
    circle: InteriorOf<Circle>,
    radial_bins: usize,
    angular_bins: usize,
}

impl CircleInteriorBins {
    /// Create a new discretized distribution for the interior of the given `circle`, with `radial_bins` bins
    /// distributed radially in concentric annuli and `angular_bins` bins distributed angularly as "pie slices".
    pub fn new(circle: Circle, radial_bins: usize, angular_bins: usize) -> Self {
        Self {
            circle: InteriorOf(circle),
            radial_bins,
            angular_bins,
        }
    }
}

impl Binned<2> for CircleInteriorBins {
    type IntermediateValue = Vec2;
    type InnerDistribution = InteriorOf<Circle>;
    fn inner_dist(&self) -> Self::InnerDistribution {
        self.circle
    }

    fn bin(&self, sample: Vec2) -> Option<[usize; 2]> {
        let radius = self.circle.0.radius;
        let theta = sample.to_angle();
        let r = sample.length();

        if !r.is_finite() || !theta.is_finite() {
            None
        } else {
            let radial_bin = bin_from_range(r, 0., radius, self.radial_bins)?;
            let angular_bin = bin_from_range(theta, -PI, PI, self.angular_bins)?;
            Some([radial_bin, angular_bin])
        }
    }
}

impl WithBinDistributions<2> for CircleInteriorBins {
    fn get_bins(&self) -> [BinDistribution; 2] {
        let radii = partition_range(0., self.circle.0.radius, self.radial_bins);

        // Factor out pi here for simplicity
        let radial_areas: Vec<_> = radii.into_iter().map(|r| r * r).collect();
        let bins_radial = BinDistribution::from_cdf(radial_areas);

        let bins_angular =
            BinDistribution::from_weights(vec![1. / self.angular_bins as f32; self.angular_bins]);

        [bins_radial, bins_angular]
    }

    fn dfs(&self) -> [usize; 2] {
        [
            self.radial_bins.saturating_sub(1),
            self.angular_bins.saturating_sub(1),
        ]
    }
}

/// A discretized distribution for the boundary of a [`Circle`].
#[derive(Clone, Copy)]
pub struct CircleBoundaryBins {
    circle: BoundaryOf<Circle>,
    angular_bins: usize,
}

impl CircleBoundaryBins {
    /// Create a new discretized distribution for the boundary of the given [`Circle`], with `angular_bins` bins
    /// distributed evenly around the edge.
    pub fn new(circle: Circle, angular_bins: usize) -> Self {
        Self {
            circle: BoundaryOf(circle),
            angular_bins,
        }
    }
}

impl Binned<1> for CircleBoundaryBins {
    type IntermediateValue = Vec2;
    type InnerDistribution = BoundaryOf<Circle>;
    fn inner_dist(&self) -> Self::InnerDistribution {
        self.circle
    }

    fn bin(&self, sample: Vec2) -> Option<[usize; 1]> {
        let theta = sample.to_angle();
        if !theta.is_finite() {
            None
        } else {
            Some([bin_from_range(theta, -PI, PI, self.angular_bins)?])
        }
    }
}

impl WithBinDistributions<1> for CircleBoundaryBins {
    fn get_bins(&self) -> [BinDistribution; 1] {
        let bins_angular =
            BinDistribution::from_weights(vec![1. / self.angular_bins as f32; self.angular_bins]);
        [bins_angular]
    }

    fn dfs(&self) -> [usize; 1] {
        [self.angular_bins - 1]
    }
}

/// A discretized distribution for the interior of a [`Sphere`].
#[derive(Clone, Copy)]
pub struct SphereInteriorBins {
    sphere: InteriorOf<Sphere>,
    radial_bins: usize,
    azimuthal_bins: usize,
    polar_bins: usize,
}

impl SphereInteriorBins {
    /// Create a new discretized distribution for the interior of the given `sphere`, with `radial_bins` bins distributed
    /// radially, `azimuthal_bins` bins swept out along the azimuth, and `polar_bins` bins swept out along the polar angle.
    pub fn new(
        sphere: Sphere,
        radial_bins: usize,
        azimuthal_bins: usize,
        polar_bins: usize,
    ) -> Self {
        Self {
            sphere: InteriorOf(sphere),
            radial_bins,
            azimuthal_bins,
            polar_bins,
        }
    }
}

impl Binned<3> for SphereInteriorBins {
    type IntermediateValue = Vec3;
    type InnerDistribution = InteriorOf<Sphere>;
    fn inner_dist(&self) -> Self::InnerDistribution {
        self.sphere
    }

    fn bin(&self, value: Vec3) -> Option<[usize; 3]> {
        let radius = self.sphere.0.radius;
        let rho = value.length();
        let theta = value.xy().to_angle();
        let psi = (value.z / rho).acos();

        if !rho.is_finite() || !theta.is_finite() || !psi.is_finite() {
            None
        } else {
            let radial_bin = bin_from_range(rho, 0., radius, self.radial_bins)?;
            let azimuthal_bin = bin_from_range(theta, -PI, PI, self.azimuthal_bins)?;
            let polar_bin = bin_from_range(psi, 0., PI, self.polar_bins)?;
            Some([radial_bin, azimuthal_bin, polar_bin])
        }
    }
}

impl WithBinDistributions<3> for SphereInteriorBins {
    fn get_bins(&self) -> [BinDistribution; 3] {
        let radius = self.sphere.0.radius;

        // Factor out constant 4/3 * pi from the volume for simplicity
        let bins_radial = BinDistribution::from_cdf(
            partition_range(0., radius, self.radial_bins)
                .into_iter()
                .map(|r| r * r * r)
                .collect::<Vec<_>>(),
        );

        let bins_azimuthal = BinDistribution::from_weights(vec![
            1. / self.azimuthal_bins as f32;
            self.azimuthal_bins
        ]);

        // Factor out constant 2/3 * pi from the volume here too
        let bins_polar = BinDistribution::from_cdf(
            partition_range(0., PI, self.polar_bins)
                .into_iter()
                .map(|psi| 1. - psi.cos())
                .collect::<Vec<_>>(),
        );

        [bins_radial, bins_azimuthal, bins_polar]
    }

    fn dfs(&self) -> [usize; 3] {
        [
            self.radial_bins.saturating_sub(1),
            self.azimuthal_bins.saturating_sub(1),
            self.polar_bins.saturating_sub(1),
        ]
    }
}

/// A discretized distribution for the boundary of a [`Sphere`].
#[derive(Clone, Copy)]
pub struct SphereBoundaryBins {
    sphere: BoundaryOf<Sphere>,
    azimuthal_bins: usize,
    polar_bins: usize,
}

impl SphereBoundaryBins {
    /// Create a new discretized distribution for the boundary of the given `sphere`, with `azimuthal_bins`
    /// swept out along the azimuth and `polar_bins` swept out along the polar angle.
    pub fn new(sphere: Sphere, azimuthal_bins: usize, polar_bins: usize) -> Self {
        Self {
            sphere: BoundaryOf(sphere),
            azimuthal_bins,
            polar_bins,
        }
    }
}

impl Binned<2> for SphereBoundaryBins {
    type IntermediateValue = Vec3;
    type InnerDistribution = BoundaryOf<Sphere>;
    fn inner_dist(&self) -> Self::InnerDistribution {
        self.sphere
    }

    fn bin(&self, value: Vec3) -> Option<[usize; 2]> {
        let theta = value.xy().to_angle();
        let psi = value.z.acos();

        if !theta.is_finite() || !psi.is_finite() {
            None
        } else {
            let azimuthal_bin = bin_from_range(theta, -PI, PI, self.azimuthal_bins)?;
            let polar_bin = bin_from_range(psi, 0., PI, self.polar_bins)?;
            Some([azimuthal_bin, polar_bin])
        }
    }
}

impl WithBinDistributions<2> for SphereBoundaryBins {
    fn get_bins(&self) -> [BinDistribution; 2] {
        let bins_azimuthal = BinDistribution::from_weights(vec![
            1. / self.azimuthal_bins as f32;
            self.azimuthal_bins
        ]);

        // Factor out 2 * pi from the surface area here for simplicity
        let bins_polar = BinDistribution::from_cdf(
            partition_range(0., PI, self.polar_bins)
                .into_iter()
                .map(|psi| 1. - psi.cos())
                .collect::<Vec<_>>(),
        );

        [bins_azimuthal, bins_polar]
    }

    fn dfs(&self) -> [usize; 2] {
        [
            self.azimuthal_bins.saturating_sub(1),
            self.polar_bins.saturating_sub(1),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{stats::*, traits::BinSampler},
        *,
    };
    use rand::{distributions::Distribution, rngs::StdRng, SeedableRng};

    /// Run goodness-of-fit tests for all directional components of this distribution.
    fn test_components<const N: usize, T>(binned_dist: T, samples: usize)
    where
        T: Binned<N> + WithBinDistributions<N> + Copy,
    {
        let rng = StdRng::from_entropy();
        let histogram: Histogram<N> = BinSampler(binned_dist)
            .sample_iter(rng)
            .take(samples)
            .collect();
        assert!(histogram.is_clean());
        let bin_dists = binned_dist.get_bins();
        let dfs = binned_dist.dfs();
        for i in 0..N {
            let chi_squared = chi_squared_fit(&histogram.project(i).unwrap(), &bin_dists[i]);
            assert!(
                chi_squared < CHI_SQUARED_CRIT_VALUES_EMINUS3[dfs[i]],
                "Goodness of fit test failed at index {i} at 0.001 significance level"
            );
        }
    }

    /// Run independence tests for each pair of directional components of this distribution.
    fn test_independence<const N: usize, T>(binned_dist: T, samples: usize)
    where
        T: Binned<N> + WithBinDistributions<N> + Copy,
    {
        let rng = StdRng::from_entropy();
        let histogram: Histogram<N> = BinSampler(binned_dist)
            .sample_iter(rng)
            .take(samples)
            .collect();
        assert!(histogram.is_clean()); // This is cheap, so we do it here as well
        let bin_dists = binned_dist.get_bins();
        let dfs = binned_dist.dfs();
        for i in 0..N {
            for j in (i + 1)..N {
                let chi_squared = chi_squared_independence(
                    &histogram.project_two([i, j]).unwrap(),
                    &[bin_dists[i].clone(), bin_dists[j].clone()],
                );
                assert!(
                    chi_squared < CHI_SQUARED_CRIT_VALUES_EMINUS3[dfs[i] * dfs[j]],
                    "Independence test failed at indices [{i}, {j}] at 0.001 significance level"
                );
            }
        }
    }

    // Independence tests only get run for sample spaces above dimension 1, and should not be implemented
    // universally; geometric properties of spheres and circles guarantee that the chosen directions should
    // *actually* be independent (constant mass "under" each angle, for example).

    // The tests are marked with #[ignore] so that they do not get run as part of CI testing.
    // They can be run by passing `-- --ignored` to `cargo test`.

    #[ignore]
    #[test]
    fn circle_interior() {
        let circle_binned = CircleInteriorBins::new(Circle::new(1.0), 8, 8);
        test_components(circle_binned, 100000);
        test_independence(circle_binned, 100000);
    }

    #[ignore]
    #[test]
    fn circle_boundary() {
        let circle_binned = CircleBoundaryBins::new(Circle::new(1.0), 20);
        test_components(circle_binned, 100000);
    }

    #[ignore]
    #[test]
    fn sphere_interior() {
        let sphere_binned = SphereInteriorBins::new(Sphere::new(1.0), 5, 8, 8);
        test_components(sphere_binned, 100000);
        test_independence(sphere_binned, 100000);
    }

    #[ignore]
    #[test]
    fn sphere_boundary() {
        let sphere_binned = SphereBoundaryBins::new(Sphere::new(1.0), 8, 8);
        test_components(sphere_binned, 100000);
        test_independence(sphere_binned, 100000);
    }
}
