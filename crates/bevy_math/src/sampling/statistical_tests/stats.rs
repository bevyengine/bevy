use super::traits::BinDistribution;
use std::collections::BTreeMap;
use thiserror::Error;

//------------//
// Histograms //
//------------//

/// An `N`-dimensional histogram, holding data simultaneously assessed to lie in
/// `N` different families of bins.
///
/// Constructed via its [`FromIterator`] implementation, hence by calling [`Iterator::collect`]
/// on an iterator whose items are of type `Option<[usize; N]>`. Most notably, the sample iterator
/// of [`BinSampler<T>`](super::traits::BinSampler) where `T` implements [`Binned`](super::traits::Binned) produces values of this type.
pub struct Histogram<const N: usize> {
    /// The actual histogram, with the invalid items diverted to `invalid`
    pub(crate) inner: BTreeMap<[usize; N], usize>,

    /// The total samples present in the histogram — i.e., excluding invalid items.
    pub total: usize,

    /// Count of invalid items, separate from the actual histogram.
    pub invalid_count: usize,
}

impl<const N: usize> FromIterator<Option<[usize; N]>> for Histogram<N> {
    fn from_iter<T: IntoIterator<Item = Option<[usize; N]>>>(iter: T) -> Self {
        let mut hist = BTreeMap::new();
        let mut total = 0;
        let mut invalid_count = 0;

        for sample in iter.into_iter() {
            let Some(sample) = sample else {
                invalid_count += 1;
                continue;
            };

            hist.entry(sample).and_modify(|v| *v += 1).or_insert(1);
            total += 1;
        }

        Self {
            inner: hist,
            total,
            invalid_count,
        }
    }
}

/// An error that is thrown when trying to use one or more invalid indices into a [`Histogram`].
#[derive(Debug, Error)]
#[error("One or more provided dimensions {dimensions:?} was outside of the range 0..{ambient_dimension}")]
pub struct InvalidDimensionError {
    ambient_dimension: usize,
    dimensions: Vec<usize>,
}

impl<const N: usize> Histogram<N> {
    /// Get the height of the histogram at a given index.
    pub fn get(&self, index: [usize; N]) -> usize {
        self.inner.get(&index).copied().unwrap_or(0)
    }

    /// Ascertain whether the [`Histogram`] is free of invalid samples.
    pub fn is_clean(&self) -> bool {
        self.invalid_count == 0
    }

    /// Project this histogram down to a histogram of dimension one by projecting away the other dimensions.
    /// Returns an [`InvalidDimensionError`] if the `dimension` parameter is outside of `0..N`.
    pub fn project(&self, dimension: usize) -> Result<Histogram<1>, InvalidDimensionError> {
        if !(0..N).contains(&dimension) {
            return Err(InvalidDimensionError {
                ambient_dimension: N,
                dimensions: vec![dimension],
            });
        }

        let mut proj_hist = BTreeMap::new();

        // The `dimension` is the fixed index; the other indices vary.
        for (&index, &height) in self.inner.iter() {
            let proj_index = index[dimension];
            proj_hist
                .entry([proj_index])
                .and_modify(|v| *v += height)
                .or_insert(height);
        }

        Ok(Histogram {
            inner: proj_hist,
            total: self.total,
            invalid_count: self.invalid_count,
        })
    }

    /// Project this histogram down to a histogram of dimension two by projecting away the other dimensions.
    /// Returns an [`InvalidDimensionError`] if any of the given `dimensions` are outside of `0..N`.
    pub fn project_two(
        &self,
        dimensions: [usize; 2],
    ) -> Result<Histogram<2>, InvalidDimensionError> {
        if !(0..N).contains(&dimensions[0]) || !(0..N).contains(&dimensions[1]) {
            return Err(InvalidDimensionError {
                ambient_dimension: N,
                dimensions: dimensions.into(),
            });
        }

        let mut proj_hist = BTreeMap::new();

        // The `dimensions` are fixed; the other indices vary.
        for (&index, &height) in self.inner.iter() {
            let proj_index = [index[dimensions[0]], index[dimensions[1]]];
            proj_hist
                .entry(proj_index)
                .and_modify(|v| *v += height)
                .or_insert(height);
        }

        Ok(Histogram {
            inner: proj_hist,
            total: self.total,
            invalid_count: self.invalid_count,
        })
    }
}

//------------//
// Statistics //
//------------//

/// Compute the chi-squared goodness-of-fit test statistic for the `histogram` relative to the ideal
/// distribution described by `ideal`. Note that this is distinct from the p-value, which must be
/// assessed separately.
pub fn chi_squared_fit(histogram: &Histogram<1>, ideal: &BinDistribution) -> f64 {
    // The vector giving the number of hits expected in each bin for this many samples
    let expecteds: Vec<_> = ideal
        .bins
        .iter()
        .map(|p| (histogram.total as f64) * (*p as f64))
        .collect();

    let mut chi_squared: f64 = 0.0;
    for (i, expected) in expecteds.into_iter().enumerate() {
        let observed = histogram.get([i]) as f64;
        let contribution = ((observed - expected) * (observed - expected)) / expected;
        chi_squared += contribution;
    }

    chi_squared
}

/// Compute the chi-squared independence test statistic for the `histogram` relative to the ideal
/// distributions described by `ideal`. Note that this is distinct from the p-value, which must be
/// assessed separately.
pub fn chi_squared_independence(histogram: &Histogram<2>, ideal: &[BinDistribution; 2]) -> f64 {
    // Compute what the expected number of hits in each bin would be based on the total samples.
    let mut expecteds: BTreeMap<[usize; 2], f64> = BTreeMap::new();
    let [dist1, dist2] = ideal;
    let (bins1, bins2) = (&dist1.bins, &dist2.bins);
    let total_samples = histogram.total as f64;
    for (i, &bin1) in bins1.iter().enumerate() {
        for (j, &bin2) in bins2.iter().enumerate() {
            expecteds.insert([i, j], bin1 as f64 * bin2 as f64 * total_samples);
        }
    }

    let mut chi_squared: f64 = 0.0;
    for (index, expected) in expecteds.into_iter() {
        let observed = histogram.get(index) as f64;
        let contribution = ((observed - expected) * (observed - expected)) / expected;
        chi_squared += contribution;
    }

    chi_squared
}

//-----------------//
// Critical Values //
//-----------------//

/// Critical values of the chi-squared distribution for ascending degrees of freedom at an alpha
/// value of 0.001. This has been shifted by one so that the index corresponds to the degrees
/// of freedom exactly; as a result, index zero is just a placeholder value.
///
/// Source: [NIST](https://www.itl.nist.gov/div898/handbook/eda/section3/eda3674.htm)
pub const CHI_SQUARED_CRIT_VALUES_EMINUS3: [f64; 101] = [
    0.0, 10.828, 13.816, 16.266, 18.467, 20.515, 22.458, 24.322, 26.125, 27.877, 29.588, 31.264,
    32.91, 34.528, 36.123, 37.697, 39.252, 40.79, 42.312, 43.82, 45.315, 46.797, 48.268, 49.728,
    51.179, 52.62, 54.052, 55.476, 56.892, 58.301, 59.703, 61.098, 62.487, 63.87, 65.247, 66.619,
    67.985, 69.347, 70.703, 72.055, 73.402, 74.745, 76.084, 77.419, 78.75, 80.077, 81.4, 82.72,
    84.037, 85.351, 86.661, 87.968, 89.272, 90.573, 91.872, 93.168, 94.461, 95.751, 97.039, 98.324,
    99.607, 100.888, 102.166, 103.442, 104.716, 105.988, 107.258, 108.526, 109.791, 111.055,
    112.317, 113.577, 114.835, 116.092, 117.346, 118.599, 119.85, 121.1, 122.348, 123.594, 124.839,
    126.083, 127.324, 128.565, 129.804, 131.041, 132.277, 133.512, 134.746, 135.978, 137.208,
    138.438, 139.666, 140.893, 142.119, 143.344, 144.567, 145.789, 147.01, 148.23, 149.449,
];

#[cfg(test)]
mod tests {
    use super::{chi_squared_fit, chi_squared_independence, BinDistribution, Histogram};
    use std::collections::BTreeMap;

    const SAMPLES_2D: [Option<[usize; 2]>; 6] = [
        Some([0, 1]),
        Some([4, 2]),
        None,
        Some([3, 3]),
        None,
        Some([4, 2]),
    ];

    const SAMPLES_3D: [Option<[usize; 3]>; 7] = [
        Some([0, 3, 5]),
        None,
        Some([7, 6, 2]),
        Some([0, 6, 3]),
        Some([1, 3, 5]),
        Some([3, 6, 2]),
        None,
    ];

    #[test]
    fn histogram_data() {
        let histogram: Histogram<2> = SAMPLES_2D.into_iter().collect();
        assert_eq!(histogram.get([4, 2]), 2);
        assert_eq!(histogram.get([0, 1]), 1);
        assert_eq!(histogram.get([3, 3]), 1);
        assert_eq!(histogram.get([0, 0]), 0);
        assert!(!histogram.is_clean());
        assert_eq!(histogram.invalid_count, 2);
        assert_eq!(histogram.total, 4);
    }

    #[test]
    fn histogram_projection() {
        let histogram: Histogram<2> = SAMPLES_2D.into_iter().collect();

        // Compare the first projection histogram to the histogram formed by the
        // collection of projected data.
        let hist_proj = histogram.project(0).unwrap();
        let hist_proj_direct: Histogram<1> = SAMPLES_2D
            .into_iter()
            .map(|opt| opt.map(|[a, _]| [a]))
            .collect();

        assert_eq!(hist_proj.invalid_count, hist_proj_direct.invalid_count);
        assert_eq!(hist_proj.total, hist_proj_direct.total);
        assert_eq!(hist_proj.inner, hist_proj_direct.inner);
    }

    #[test]
    fn histogram_project_two() {
        let histogram: Histogram<3> = SAMPLES_3D.into_iter().collect();

        // Compare the [1, 2]-projection histogram to the histogram formed by the
        // collection of projected data.
        let hist_proj = histogram.project_two([1, 2]).unwrap();
        let hist_proj_direct: Histogram<2> = SAMPLES_3D
            .into_iter()
            .map(|opt| opt.map(|[_, b, c]| [b, c]))
            .collect();

        assert_eq!(hist_proj.invalid_count, hist_proj_direct.invalid_count);
        assert_eq!(hist_proj.total, hist_proj_direct.total);
        assert_eq!(hist_proj.inner, hist_proj_direct.inner);
    }

    #[test]
    fn histogram_project_two_duplicate() {
        let histogram: Histogram<2> = SAMPLES_2D.into_iter().collect();

        // Verify that diagonal projections work how one would expect.
        let hist_proj: Histogram<2> = histogram.project_two([1, 1]).unwrap();
        let hist_proj_direct: Histogram<2> = SAMPLES_2D
            .into_iter()
            .map(|opt| opt.map(|[_, b]| [b, b]))
            .collect();

        assert_eq!(hist_proj.invalid_count, hist_proj_direct.invalid_count);
        assert_eq!(hist_proj.total, hist_proj_direct.total);
        assert_eq!(hist_proj.inner, hist_proj_direct.inner);
    }

    #[test]
    fn chi_squared_goodness() {
        let histogram = SAMPLES_2D
            .into_iter()
            .collect::<Histogram<2>>()
            .project(0)
            .unwrap();

        // Uniform distribution on 5 bins 0..5
        let ideal = BinDistribution::from_weights(vec![1.; 5]);
        let chi_squared = chi_squared_fit(&histogram, &ideal);
        assert!((chi_squared - 3.5).abs() < 1e-7);
    }

    #[test]
    fn chi_squared_indep() {
        let mut dist: BTreeMap<[usize; 2], usize> = BTreeMap::new();
        dist.insert([0, 0], 3);
        dist.insert([0, 1], 2);
        dist.insert([1, 0], 0);
        dist.insert([1, 1], 1);

        let histogram = Histogram {
            inner: dist,
            total: 6,
            invalid_count: 0,
        };

        let ideal_first = BinDistribution::from_weights(vec![1.; 2]);
        let ideal_second = BinDistribution::from_weights(vec![1.; 2]);
        let chi_squared = chi_squared_independence(&histogram, &[ideal_first, ideal_second]);
        assert!((chi_squared - 10. / 3.).abs() < 1e-7);
    }
}
