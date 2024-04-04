//! A specialized library for performing statistical testing on the quality of
//! spatial distributions.
//!
//! Included are traits for binning statistical distributions in space ([`traits`]),
//! comparing those binned distributions to ideal multinomial distributions across the
//! sets of bins ([`stats`]), and concrete implementations of these for Bevy's shape
//! types ([`impls`]).

/// Holds traits [`Binned`](traits::Binned) and [`WithBinDistributions`](traits::WithBinDistributions), which form the scaffolding for
/// discretization of spatial distributions (with the former) and comparison of the resulting
/// discrete probability distributions with ideal multinomial distributions (with the latter).
pub mod traits;

/// Holds the [`Histogram`](stats::Histogram) type, which is an `N`-dimensional histogram that can be accumulated
/// from the distributions constructed with implementations of [`Binned`](traits::Binned). Also holds
/// the chi-squared goodness-of-fit and independence tests that are used to assess the quality
/// of binned distributions.
pub mod stats;

/// Holds concrete implementations of this library's [`traits`] for spatial distributions derived
/// from Bevy's primitive shapes, along with tests utilizing those to verify the statistical
/// quality of those distributions.
pub mod impls;
