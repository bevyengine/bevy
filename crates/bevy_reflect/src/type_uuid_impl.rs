use crate as bevy_reflect;
use bevy_reflect_derive::impl_type_uuid;
use bevy_utils::{Duration, HashMap, HashSet, Instant};
use smallvec::SmallVec;
use std::{
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    ops::{RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    path::PathBuf,
};

impl_type_uuid!(bool, 0xeb1ad0ee2dff473285bc54ebbdef682c);
impl_type_uuid!(char, 0x45a4710278ba48f8b31f0d72ff7f9d46);
impl_type_uuid!(u8, 0xfdf1a88a3e0543ca9f51ad5978ca519f);
impl_type_uuid!(u16, 0xddeb93f791074860aaac1540de254edc);
impl_type_uuid!(u32, 0xfc565ea2367f405591e1c55f91cb60bd);
impl_type_uuid!(u64, 0x6c74b6a983eb44b096a9169baa6af0a1);
impl_type_uuid!(u128, 0xf837371a4f534b7381ed776d5056d0c1);
impl_type_uuid!(usize, 0x0129e1d8cff041f9b23aa99c6e1006b8);
impl_type_uuid!(i8, 0xaf7a5411661e43b0b1631ea43a825fd2);
impl_type_uuid!(i16, 0x68592d5de5be4a608603c6988edfdf9c);
impl_type_uuid!(i32, 0x439ff07f96c94aa5a86352ded71e4730);
impl_type_uuid!(i64, 0x7f9534793ad24ab2b9f05d8254f4204a);
impl_type_uuid!(i128, 0x6e5009be5845460daf814e052cc9fcf0);
impl_type_uuid!(isize, 0xd3d52630da45497faf86859051c79e7d);
impl_type_uuid!(f32, 0x006607124a8148e1910c86f0c18c9015);
impl_type_uuid!(f64, 0xa5bc32f5632b478c92a0939b821fff80);
impl_type_uuid!(Result<T, E>, 0xd5960af2e8a743dfb7427dd59b70df95);
impl_type_uuid!(String, 0xc9f90d31b52d4bcd8b5c1d8b6fc1bcba);
impl_type_uuid!(PathBuf, 0xaa79933abd1743698583a3acad3b8989);
impl_type_uuid!(Vec<T>, 0xab98f5408b974475b643662247fb3886);
impl_type_uuid!(HashMap<K, V>,0xf37bfad9ca8c4f6ea7448f1c39e05f98 );
impl_type_uuid!(Option<T>, 0x8d5ba9a9031347078955fba01ff439f0);
impl_type_uuid!(
    SmallVec<T: smallvec::Array>,
    0x26fd5c1bed7144fbb8d1546c02ba255a
);
impl_type_uuid!(HashSet<K>, 0x5ebd2379ece44ef2b1478262962617a3);
impl_type_uuid!(RangeInclusive<T>, 0x79613b729ca9490881c7f47b24b22b60);
impl_type_uuid!(RangeFrom<T>, 0x1bd8c975f122486c9ed443e277964642);
impl_type_uuid!(RangeTo<T>, 0x7d938903749a4d198f496cb354929b9b);
impl_type_uuid!(RangeToInclusive<T>, 0x2fec56936206462fa5f35c99a62c5ed1);
impl_type_uuid!(RangeFull, 0x227af17f65db448782a2f6980ceae25d);
impl_type_uuid!(Duration, 0xcee5978c60f74a53b6848cb9c46a6e1c);
impl_type_uuid!(Instant, 0x9b0194a1d31c44c1afd2f6fd80ab8dfb);
impl_type_uuid!(NonZeroI128, 0x915a1e7fcaeb433982cebf58c2ac20e7);
impl_type_uuid!(NonZeroU128, 0x286de521146042cda31dfbef8f3f6cdc);
impl_type_uuid!(NonZeroIsize, 0x9318740a9fd14603b709b8fbc6fd2812);
impl_type_uuid!(NonZeroUsize, 0xa26533ed16324189878263d5e7a294ce);
impl_type_uuid!(NonZeroI64, 0x1aa38623127a42419cca4992e6fc3152);
impl_type_uuid!(NonZeroU64, 0x46be65e669a2477d942e2ec39d0d2af7);
impl_type_uuid!(NonZeroU32, 0xcf53a46d9efe4022967160cb61762c91);
impl_type_uuid!(NonZeroI32, 0xa69fbd659bef4322b88b15ff3263f530);
impl_type_uuid!(NonZeroI16, 0x8744c2ec8a10491fae40f8bafa58b30d);
impl_type_uuid!(NonZeroU16, 0xc7b8b60780a6495bab4fda2bdfedabcc);
impl_type_uuid!(NonZeroU8, 0x635ee104ef7947fb9d7f79dad47255a3);
impl_type_uuid!(NonZeroI8, 0x2d3f1570b7f64779826d44da5c7ba069);
