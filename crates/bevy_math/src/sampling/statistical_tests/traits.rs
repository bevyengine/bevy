use rand::distributions::Distribution;
use rand::Rng;

/// A trait implemented by a type which discretizes the sample space of a [`Distribution`] simultaneously
/// in `N` dimensions. To sample an implementing type as a [`Distribution`], use the [`BinSampler`] wrapper
/// type.
pub trait Binned<const N: usize> {
    /// The type defining the sample space discretized by this type.
    type IntermediateValue;

    /// The inner distribution type whose samples are to be discretized.
    type InnerDistribution: Distribution<Self::IntermediateValue>;

    /// The concrete inner distribution of this distribution, used to sample into an `N`-dimensional histogram.
    fn inner_dist(&self) -> Self::InnerDistribution;

    /// A function that takes output from the inner distribution and maps it to `N` bins. This allows
    /// any implementor of `Binned` to be a [`Distribution`] — the output of the distribution is `Option<[usize; N]>`
    /// because the mapping to bins is generally fallible, resulting in an error state when a sample misses every bin.
    fn bin(&self, value: Self::IntermediateValue) -> Option<[usize; N]>;

    /// Bin-sample the discretized distribution.
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<[usize; N]> {
        let v = self.inner_dist().sample(rng);
        self.bin(v)
    }
}

#[derive(Debug, Clone)]
/// A collection of bins with weights that define a discrete probability distribution over the indices.
/// That is, a formal description of a multinomial distribution, equipped with methods to aid in its
/// construction.
pub struct BinDistribution {
    /// The underlying bin weights of this distribution.
    pub bins: Vec<f32>,
}

impl BinDistribution {
    /// Construct a new [`BinDistribution`] from its sequence of weights. This function normalizes
    /// the resulting distribution, so the only thing that matters for the caller is that the `weights`
    /// have the correct proportions relative to one another.
    pub fn from_weights(weights: impl Into<Vec<f32>>) -> Self {
        let bins = Self {
            bins: weights.into(),
        };
        bins.normalized()
    }

    /// Construct a new [`BinDistribution`] from a (potentially non-normalized) sequence of cumulative weights.
    /// Like [`BinDistribution::from_weights`], the resulting distribution is normalized automatically, so the
    /// caller need ensure only that the cumulative weights are correctly proportioned.
    pub fn from_cdf(cdf_weights: impl Into<Vec<f32>>) -> Self {
        let cdf: Vec<f32> = cdf_weights.into();
        let mut pdf = Vec::with_capacity(cdf.len());

        // Convert cdf to pdf by subtracting adjacent elements
        pdf.push(cdf[0]);
        for window in cdf.as_slice().windows(2) {
            pdf.push(window[1] - window[0]);
        }

        BinDistribution::from_weights(pdf)
    }

    /// Normalize the bin data inside this distribution.
    fn normalize(&mut self) {
        let total: f32 = self.bins.iter().copied().sum();
        if total.is_normal() {
            self.bins.iter_mut().for_each(|p| *p /= total);
        }
    }

    /// This distribution, but with its interior bin data normalized.
    fn normalized(mut self) -> Self {
        self.normalize();
        self
    }
}

/// A discretized ([`Binned`]) probability distribution that also has extrinsic weights associated to its bins;
/// primarily intended for use in chi-squared analysis of spatial distributions.
pub trait WithBinDistributions<const N: usize>: Binned<N> {
    /// Get the bin weights to compare with actual samples.
    fn get_bins(&self) -> [BinDistribution; N];

    /// Get the degrees of freedom of each set of bins.
    fn dfs(&self) -> [usize; N] {
        self.get_bins().map(|b| b.bins.len().saturating_sub(1))
    }
}

/// A wrapper struct that allows a [`Binned`] distribution type to be used directly as a [`Distribution`].
pub struct BinSampler<const N: usize, T: Binned<N>>(pub T);

impl<const N: usize, T: Binned<N>> Distribution<Option<[usize; N]>> for BinSampler<N, T> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<[usize; N]> {
        Binned::sample(&self.0, rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bin_normalization() {
        let weights = vec![2.0, 3.5, 1.0];
        let bins = BinDistribution::from_weights(weights);
        assert_eq!(bins.bins[0], 2.0 / 6.5);
        assert_eq!(bins.bins[1], 3.5 / 6.5);
        assert_eq!(bins.bins[2], 1.0 / 6.5);
    }

    #[test]
    fn bin_cdf() {
        let cdf = [1., 2., 3., 4., 5.];
        let bins = BinDistribution::from_cdf(cdf);
        for i in 0..cdf.len() {
            assert_eq!(bins.bins[i], 0.2);
        }
    }
}
