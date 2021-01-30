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

    fn as_slice(&self) -> &[Self::Item];

    fn as_slice_mut(&mut self) -> &mut [Self::Item];
}

macro_rules! arrayn(
    ($($size:expr),+) => {
        $(
            unsafe impl<T> ArrayN for [T; $size] {
                type Item = T;
                fn size() -> usize { $size }
                fn get(&self, index: usize) -> &Self::Item { &self[index] }
                fn get_mut(&mut self, index: usize) -> &mut Self::Item { &mut self[index] }
                fn as_slice(&self) -> &[Self::Item] { &self[..] }
                fn as_slice_mut(&mut self) -> &mut [Self::Item] { &mut self[..] }
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
    type Lanes: ArrayN<Item = u16>;

    /// [Self::Value; N]
    type Outputs: ArrayN<Item = Self::Value>;

    fn unpack(self) -> Self::Outputs;

    /// N
    fn size() -> usize;
}

/// Tack with N output lanes
pub trait TrackN {
    type Output;

    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Output)) -> u16;

    fn duration(&self) -> f32;

    fn lanes(&self) -> &[u16];

    /// N
    fn size(&self) -> usize;
}

pub struct TrackNData<V: ValueN, T: Track<Output = V>> {
    pub lanes: V::Lanes,
    /// Only applicable when `outputs.len() > 0`, defines how many of the output lanes are actually assigned;
    /// In the case of `len == 0` this track doesn't output anything and should be deleted to preserve performance
    pub len: u16,
    pub track: T,
}

impl<V: ValueN, T: Track<Output = V>> TrackN for TrackNData<V, T> {
    type Output = V::Value;

    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Output)) -> u16 {
        let (cursor, x) = self.track.sample_with_cursor(cursor, time);
        let x = x.unpack();
        for i in 0..(self.len as usize).min(self.size()) {
            (assign)(*self.lanes.get(i), *x.get(i));
        }
        cursor
    }

    fn duration(&self) -> f32 {
        self.track.duration()
    }

    fn lanes(&self) -> &[u16] {
        self.lanes.as_slice()
    }

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

pub type TrackNBase<T> = Box<dyn TrackN<Output = T> + Send + Sync + 'static>;

// impl<T> Sampler for SamplerBase<T> {
//     type Out = T;

//     #[inline(always)]
//     fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Output)) -> u16 {
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

            type Lanes = [u16; 1];

            type Outputs = [Self::Value; 1];

            fn unpack(self) -> Self::Outputs {
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

            type Lanes = [u16; $s];

            type Outputs = [Self::Value; $s];

            #[inline(always)]
            fn unpack(self) -> Self::Outputs {
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

impl ValueN for Quatx4 {
    type Value = Quat;

    type Lanes = [u16; 4];

    type Outputs = [Self::Value; 4];

    #[inline(always)]
    fn unpack(self) -> Self::Outputs {
        unsafe { transmute::<[ultraviolet::Vec4; 4], _>(self.0.into()) }
    }

    fn size() -> usize {
        4
    }
}

impl ValueN for Quatx8 {
    type Value = Quat;

    type Lanes = [u16; 8];

    type Outputs = [Self::Value; 8];

    #[inline(always)]
    fn unpack(self) -> Self::Outputs {
        unsafe { transmute::<[ultraviolet::Vec4; 8], _>(self.0.into()) }
    }

    fn size() -> usize {
        8
    }
}
