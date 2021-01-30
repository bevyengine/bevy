use std::mem::transmute;

use bevy_math::prelude::*;
use bevy_render::prelude::Color;

use crate::{
    interpolate::Lerp,
    tracks::{Track, TrackFixed},
    wide::*,
};

///////////////////////////////////////////////////////////////////////////////

// ? NOTE: Index and IndexMut are only available in rust 1.50.0

/// Array of N elements
pub unsafe trait ArrayN {
    type Item;

    fn size() -> usize;

    fn get(&self, index: usize) -> &Self::Item;

    fn get_mut(&mut self, index: usize) -> &mut Self::Item;
}

macro_rules! arrayn(
    ($($size:expr),+) => {
        $(
            unsafe impl<T> ArrayN for [T; $size] {
                type Item = T;
                fn size() -> usize { $size }
                fn get(&self, index: usize) -> &Self::Item { &self[index] }
                fn get_mut(&mut self, index: usize) -> &mut Self::Item { &mut self[index] }
            }
        )+
    }
);

arrayn!(1, 4, 8);

///////////////////////////////////////////////////////////////////////////////

/// N length value
pub trait ValueN {
    type Value: Copy;

    /// [u16; N]
    type Outputs: ArrayN<Item = u16>;

    /// [Self::Value; N]
    type Lanes: ArrayN<Item = Self::Value>;

    fn unpack(self) -> Self::Lanes;

    /// N
    fn size() -> usize;
}

/// Tack with N output lanes
pub trait TrackN {
    type Out;

    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Out)) -> u16;

    /// N
    fn size(&self) -> usize;
}

pub struct TrackNData<V: ValueN, T: Track<Out = V>> {
    /// These indexes are usually entity indexes within the clip space, a fully defined channel is
    /// made by a property name plus some index, each track will be sorted by property
    pub outputs: V::Outputs,
    /// Only applicable when `channels.len() > 0`, defines how many of the output lanes are actually assigned;
    /// In the case of `len == 0` this track doesn't output anything and should be deleted to preserve performance
    pub len: u16,
    pub track: T,
}

impl<V: ValueN, T: Track<Out = V>> TrackN for TrackNData<V, T> {
    type Out = V::Value;

    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Out)) -> u16 {
        let (cursor, x) = self.track.sample_with_cursor(cursor, time);
        let x = x.unpack();
        for i in 0..(self.len as usize).min(self.size() - 1) {
            (assign)(*self.outputs.get(i), *x.get(i));
        }
        cursor
    }

    #[inline(always)]
    fn size(&self) -> usize {
        V::size()
    }
}

impl<V: ValueN + Lerp + Clone> TrackNData<V, TrackFixed<V>> {
    // TODO: Function to edit each lane
    // pub fn add_track
    // pub fn add_track_resampled (will accept any kind of track)
    // pub fn remove_track
}

pub type TrackNBase<T> = Box<dyn TrackN<Out = T> + Send + Sync + 'static>;

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

///////////////////////////////////////////////////////////////////////////////

macro_rules! output1x1 {
    ($t:ty) => {
        impl ValueN for $t {
            type Value = $t;

            type Outputs = [u16; 1];

            type Lanes = [Self::Value; 1];

            fn unpack(self) -> Self::Lanes {
                [self]
            }

            fn size() -> usize {
                1
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
        impl ValueN for $t {
            type Value = $o;

            type Outputs = [u16; $s];

            type Lanes = [Self::Value; $s];

            #[inline(always)]
            fn unpack(self) -> Self::Lanes {
                unsafe { transmute::<[$i; $s], _>(self.into()) }
            }

            fn size() -> usize {
                $s
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

// impl SamplerValue for Quatx4 {
//     type Out = [Quat; 4];
//     type Indexes = [u16; 4];

//     #[inline(always)]
//     fn output(self) -> Self::Out {
//         unsafe { transmute::<[ultraviolet::Vec4; 4], _>(self.0.into()) }
//     }
// }

// impl SamplerValue for Quatx8 {
//     type Out = [Quat; 8];
//     type Indexes = [u16; 8];

//     #[inline(always)]
//     fn output(self) -> Self::Out {
//         unsafe { transmute::<[ultraviolet::Vec4; 8], _>(self.0.into()) }
//     }
// }
