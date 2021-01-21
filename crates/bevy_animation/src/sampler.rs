use std::mem::transmute;

use bevy_math::prelude::*;
use bevy_render::prelude::Color;
use smallvec::Array;

use crate::{tracks::Track, wide::*};

/// Defines a sampler value output;
///
/// Required because wide types outputs to many different channels at the same time
pub trait SamplerValue {
    /// The result value ready to be assign to each channel;
    ///
    /// Are defined as `Self` for simpler types like `f32` or `Vec3` etc, and arrays for wide types like `Vec3x4`;
    type Out;

    /// Defines how many channels needed be indexed in oder to assign all of this output
    type Indexes: Array<Item = u16>;

    fn output(self) -> Self::Out;
}

/// Defines a sampler and it's target channels
pub trait Sampler {
    /// Channel type
    type Out;

    /// Evaluates the sampler while assign each outputs to their respective channel
    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Out)) -> u16;
}

/// Defines how to sample an track and output it's results
pub struct TrackSampler<V: SamplerValue, T: Track> {
    /// These indexes are usually entity indexes within the clip space, a fully defined channel is
    /// made by a property name plus some index, each track will be sorted by property
    pub channels: V::Indexes,
    /// Only applicable when `channels.len() > 0`, defines how many of the output lanes are actually assigned;
    /// In the case of `len == 0` this track doesn't output anything and should be deleted to preserve performance
    pub len: u16,
    pub track: T,
}

// /// Base sampler type
// pub struct SamplerBase<T>(Box<dyn Sampler<Out = T> + Send + Sync + 'static>);

// impl<T> Sampler for SamplerBase<T> {
//     type Out = T;

//     #[inline(always)]
//     fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Out)) -> u16 {
//         self.0.sample(cursor, time, assign)
//     }
// }

// pub struct SamplerUntyped(Box<dyn Any + Send + Sync + 'static>);

// impl SamplerUntyped {
//     pub fn new<S>(sampler: S) -> Self
//     where
//         S: Sampler + Send + Sync + 'static,
//     {
//         SamplerUntyped(Box::new(SamplerBase(Box::new(sampler))))
//     }

//     #[inline(always)]
//     pub fn downcast_ref<T: 'static>(&self) -> Option<&SamplerBase<T>> {
//         self.0.downcast_ref()
//     }

//     #[inline(always)]
//     pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut SamplerBase<T>> {
//         self.0.downcast_mut()
//     }
// }

macro_rules! output1x1 {
    ($t:ty) => {
        impl SamplerValue for $t {
            type Out = $t;
            type Indexes = [u16; 1];

            #[inline(always)]
            fn output(self) -> Self::Out {
                self
            }
        }

        impl<T: Track<Out = $t>> Sampler for TrackSampler<$t, T> {
            type Out = $t;

            fn sample(
                &self,
                cursor: u16,
                time: f32,
                assign: &mut dyn FnMut(u16, Self::Out),
            ) -> u16 {
                let (cursor, x) = self.track.sample_with_cursor(cursor, time);
                (assign)(self.channels[0], x);
                cursor
            }
        }
    };
}

output1x1!(bool);
output1x1!(u8);
output1x1!(u16);
output1x1!(u32);
output1x1!(u64);
output1x1!(usize);
output1x1!(i8);
output1x1!(i16);
output1x1!(i32);
output1x1!(i64);
output1x1!(isize);
output1x1!(f32);
output1x1!(f64);

output1x1!(Vec2);
output1x1!(Vec3);
output1x1!(Vec4);
output1x1!(Color);
output1x1!(Quat);

macro_rules! output1xn {
    ($t:ty, $i:ty, $o:ty, $s:tt) => {
        impl SamplerValue for $t {
            type Out = [$o; $s];
            type Indexes = [u16; $s];

            #[inline(always)]
            fn output(self) -> Self::Out {
                unsafe { transmute::<[$i; $s], _>(self.into()) }
            }
        }

        impl<T: Track<Out = $t>> Sampler for TrackSampler<$t, T> {
            type Out = $o;

            fn sample(
                &self,
                cursor: u16,
                time: f32,
                assign: &mut dyn FnMut(u16, Self::Out),
            ) -> u16 {
                let (cursor, x) = self.track.sample_with_cursor(cursor, time);
                let x = x.output();
                for i in 0..(self.len as usize).min($s - 1) {
                    (assign)(self.channels[i], x[i]);
                }
                cursor
            }
        }
    };
}

output1xn!(f32x4, f32, f32, 4);
output1xn!(f32x8, f32, f32, 8);
output1xn!(Vec3x4, ultraviolet::Vec3, Vec3, 4);
output1xn!(Vec3x8, ultraviolet::Vec3, Vec3, 8);
output1xn!(Vec4x4, ultraviolet::Vec4, Vec4, 4);
output1xn!(Vec4x8, ultraviolet::Vec4, Vec4, 8);

impl SamplerValue for Quatx4 {
    type Out = [Quat; 4];
    type Indexes = [u16; 4];

    #[inline(always)]
    fn output(self) -> Self::Out {
        unsafe { transmute::<[ultraviolet::Vec4; 4], _>(self.0.into()) }
    }
}

impl<T: Track<Out = Quatx4>> Sampler for TrackSampler<Quatx4, T> {
    type Out = Quat;

    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Out)) -> u16 {
        let (cursor, x) = self.track.sample_with_cursor(cursor, time);
        let x = x.output();
        for i in 0..(self.len as usize).min(4 - 1) {
            (assign)(self.channels[i], x[i]);
        }
        cursor
    }
}

impl SamplerValue for Quatx8 {
    type Out = [Quat; 8];
    type Indexes = [u16; 8];

    #[inline(always)]
    fn output(self) -> Self::Out {
        unsafe { transmute::<[ultraviolet::Vec4; 8], _>(self.0.into()) }
    }
}

impl<T: Track<Out = Quatx8>> Sampler for TrackSampler<Quatx8, T> {
    type Out = Quat;

    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Out)) -> u16 {
        let (cursor, x) = self.track.sample_with_cursor(cursor, time);
        let x = x.output();
        for i in 0..(self.len as usize).min(8 - 1) {
            (assign)(self.channels[i], x[i]);
        }
        cursor
    }
}
