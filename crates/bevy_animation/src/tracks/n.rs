use std::mem::transmute;

use bevy_math::prelude::*;
// TODO: `bevy_render::prelude::Color` can be implemented the same way as Quatx4
// use bevy_render::prelude::Color;

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

    fn pack(values: Self::Outputs) -> Self;

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

    /// Number of used outputs, may be N or less
    fn len(&self) -> usize;

    /// N
    fn size(&self) -> usize;
}

pub struct TrackFixedN<V: ValueN> {
    pub lanes: V::Lanes,
    /// Only applicable when `outputs.len() > 0`, defines how many of the output lanes are actually assigned;
    /// In the case of `len == 0` this track doesn't output anything and should be deleted to preserve performance
    pub len: u16,
    pub track: TrackFixed<V>,
}

impl<V: ValueN + Lerp + Clone> TrackN for TrackFixedN<V> {
    type Output = V::Value;

    fn sample(&self, cursor: u16, time: f32, assign: &mut dyn FnMut(u16, Self::Output)) -> u16 {
        let (cursor, x) = self.track.sample_with_cursor(cursor, time);
        let x = x.unpack();
        for i in 0..self.len() {
            (assign)(*self.lanes.get(i), *x.get(i));
        }
        cursor
    }

    fn duration(&self) -> f32 {
        self.track.duration()
    }

    fn len(&self) -> usize {
        (self.len as usize).min(V::size())
    }

    fn lanes(&self) -> &[u16] {
        &self.lanes.as_slice()[0..self.len()]
    }

    fn size(&self) -> usize {
        V::size()
    }
}

// impl<V: ValueN + Lerp + Clone> TrackFixedN<V> {
//     // TODO: Function to edit each lane
//     // pub fn add_track

//     pub fn add_track_resampled(&mut self, track: &dyn Track<Output = <V as ValueN>::Value>) {
//         let i = self.len as usize;
//         if i >= V::size() {
//             panic!("track is full");
//         }

//         self.len += 1;

//         let mut cursor = 0;
//         let f = 1.0 / (self.track.frame_rate() as f32);
//         let offset = self.track.offset() as f32 * f;
//         let s = self.track.len();
//         let keyframes = self.track.keyframes_mut();

//         for frame in 0..s {
//             let time = offset + f * frame as f32;
//             let (k, v) = track.sample_with_cursor(cursor, time);
//             keyframes[frame].set(v, i);
//             cursor = k;
//         }
//     }

//     // pub fn remove_track
// }

pub type TrackNBase<T> = Box<dyn TrackN<Output = T> + Send + Sync + 'static>;

///////////////////////////////////////////////////////////////////////////////

macro_rules! valuen {
    ($t:ty, $i:ty, $o:ty, $s:tt) => {
        impl ValueN for $t {
            type Lanes = [u16; $s];
            type Outputs = [Self::Value; $s];
            type Value = $o;

            #[inline]
            fn pack(values: Self::Outputs) -> Self {
                unsafe { transmute::<_, [$i; $s]>(values).into() }
            }

            #[inline]
            fn unpack(self) -> Self::Outputs {
                unsafe { transmute::<[$i; $s], _>(self.into()) }
            }

            fn size() -> usize {
                $s
            }
        }
    };
}

valuen!(f32x4, f32, f32, 4);
valuen!(f32x8, f32, f32, 8);
valuen!(Vec3x4, ultraviolet::Vec3, Vec3, 4);
valuen!(Vec3x8, ultraviolet::Vec3, Vec3, 8);
valuen!(Vec4x4, ultraviolet::Vec4, Vec4, 4);
valuen!(Vec4x8, ultraviolet::Vec4, Vec4, 8);

impl ValueN for Quatx4 {
    type Lanes = [u16; 4];
    type Outputs = [Self::Value; 4];
    type Value = Quat;

    #[inline]
    fn pack(values: Self::Outputs) -> Self {
        Quatx4(unsafe { transmute::<_, [ultraviolet::Vec4; 4]>(values).into() })
    }

    #[inline]
    fn unpack(self) -> Self::Outputs {
        unsafe { transmute::<[ultraviolet::Vec4; 4], _>(self.0.into()) }
    }

    fn size() -> usize {
        4
    }
}

impl ValueN for Quatx8 {
    type Lanes = [u16; 8];
    type Outputs = [Self::Value; 8];
    type Value = Quat;

    #[inline]
    fn pack(values: Self::Outputs) -> Self {
        Quatx8(unsafe { transmute::<_, [ultraviolet::Vec4; 8]>(values).into() })
    }

    #[inline]
    fn unpack(self) -> Self::Outputs {
        unsafe { transmute::<[ultraviolet::Vec4; 8], _>(self.0.into()) }
    }

    fn size() -> usize {
        8
    }
}
