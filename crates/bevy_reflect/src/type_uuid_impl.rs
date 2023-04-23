use crate::TypeUuid;
use crate::{self as bevy_reflect, __macro_exports::generate_composite_uuid};
use bevy_reflect_derive::impl_type_uuid;
use bevy_utils::{all_tuples, Duration, HashMap, HashSet, Instant, Uuid};
#[cfg(feature = "smallvec")]
use smallvec::SmallVec;
#[cfg(any(unix, windows))]
use std::ffi::OsString;
use std::{
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    ops::{RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    path::PathBuf,
};
impl<T: TypeUuid, const N: usize> TypeUuid for [T; N] {
    const TYPE_UUID: Uuid = generate_composite_uuid(
        Uuid::from_u128(0x18d33c78e63c47b9bbf8f095008ab693),
        generate_composite_uuid(Uuid::from_u128(N as u128), T::TYPE_UUID),
    );
}
impl_type_uuid!(bool, "eb1ad0ee2dff473285bc54ebbdef682c");
impl_type_uuid!(char, "45a4710278ba48f8b31f0d72ff7f9d46");
impl_type_uuid!(u8, "fdf1a88a3e0543ca9f51ad5978ca519f");
impl_type_uuid!(u16, "ddeb93f791074860aaac1540de254edc");
impl_type_uuid!(u32, "fc565ea2367f405591e1c55f91cb60bd");
impl_type_uuid!(u64, "6c74b6a983eb44b096a9169baa6af0a1");
impl_type_uuid!(u128, "f837371a4f534b7381ed776d5056d0c1");
impl_type_uuid!(usize, "0129e1d8cff041f9b23aa99c6e1006b8");
impl_type_uuid!(i8, "af7a5411661e43b0b1631ea43a825fd2");
impl_type_uuid!(i16, "68592d5de5be4a608603c6988edfdf9c");
impl_type_uuid!(i32, "439ff07f96c94aa5a86352ded71e4730");
impl_type_uuid!(i64, "7f9534793ad24ab2b9f05d8254f4204a");
impl_type_uuid!(i128, "6e5009be5845460daf814e052cc9fcf0");
impl_type_uuid!(isize, "d3d52630da45497faf86859051c79e7d");
impl_type_uuid!(f32, "006607124a8148e1910c86f0c18c9015");
impl_type_uuid!(f64, "a5bc32f5632b478c92a0939b821fff80");
impl_type_uuid!(Result<T, E>, "d5960af2e8a743dfb7427dd59b70df95");
impl_type_uuid!(String, "c9f90d31b52d4bcd8b5c1d8b6fc1bcba");
impl_type_uuid!(PathBuf, "aa79933abd1743698583a3acad3b8989");
impl_type_uuid!(Vec<T>, "ab98f5408b974475b643662247fb3886");
impl_type_uuid!(HashMap<K, V>,"f37bfad9ca8c4f6ea7448f1c39e05f98");
impl_type_uuid!(Option<T>, "8d5ba9a9031347078955fba01ff439f0");
#[cfg(feature = "smallvec")]
impl_type_uuid!(
    SmallVec<T: smallvec::Array>,
    "26fd5c1bed7144fbb8d1546c02ba255a"
);
impl_type_uuid!(HashSet<K>, "5ebd2379ece44ef2b1478262962617a3");
impl_type_uuid!(RangeInclusive<T>, "79613b729ca9490881c7f47b24b22b60");
impl_type_uuid!(RangeFrom<T>, "1bd8c975f122486c9ed443e277964642");
impl_type_uuid!(RangeTo<T>, "7d938903749a4d198f496cb354929b9b");
impl_type_uuid!(RangeToInclusive<T>, "2fec56936206462fa5f35c99a62c5ed1");
impl_type_uuid!(RangeFull, "227af17f65db448782a2f6980ceae25d");
impl_type_uuid!(Duration, "cee5978c60f74a53b6848cb9c46a6e1c");
impl_type_uuid!(Instant, "9b0194a1d31c44c1afd2f6fd80ab8dfb");
impl_type_uuid!(NonZeroI128, "915a1e7fcaeb433982cebf58c2ac20e7");
impl_type_uuid!(NonZeroU128, "286de521146042cda31dfbef8f3f6cdc");
impl_type_uuid!(NonZeroIsize, "9318740a9fd14603b709b8fbc6fd2812");
impl_type_uuid!(NonZeroUsize, "a26533ed16324189878263d5e7a294ce");
impl_type_uuid!(NonZeroI64, "1aa38623127a42419cca4992e6fc3152");
impl_type_uuid!(NonZeroU64, "46be65e669a2477d942e2ec39d0d2af7");
impl_type_uuid!(NonZeroU32, "cf53a46d9efe4022967160cb61762c91");
impl_type_uuid!(NonZeroI32, "a69fbd659bef4322b88b15ff3263f530");
impl_type_uuid!(NonZeroI16, "8744c2ec8a10491fae40f8bafa58b30d");
impl_type_uuid!(NonZeroU16, "c7b8b60780a6495bab4fda2bdfedabcc");
impl_type_uuid!(NonZeroU8, "635ee104ef7947fb9d7f79dad47255a3");
impl_type_uuid!(NonZeroI8, "2d3f1570b7f64779826d44da5c7ba069");
#[cfg(any(unix, windows))]
impl_type_uuid!(OsString, "809e7b3c1ea240979ecd832f91eb842a");
macro_rules! impl_tuple {
    ( $($name: ident),* ) => {
        const _: () = {
            type Tuple< $($name),* > = ( $($name,)* );
            impl_type_uuid!(Tuple< $($name),* > , "35c8a7d3d4b34bd7b8471118dc78092f");
        };
    };
}
all_tuples!(impl_tuple, 0, 12, A);
