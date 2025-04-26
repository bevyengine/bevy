use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::ops::Range<T: Clone + Send + Sync>(Clone));
impl_reflect_opaque!(::core::ops::RangeInclusive<T: Clone + Send + Sync>(Clone));
impl_reflect_opaque!(::core::ops::RangeFrom<T: Clone + Send + Sync>(Clone));
impl_reflect_opaque!(::core::ops::RangeTo<T: Clone + Send + Sync>(Clone));
impl_reflect_opaque!(::core::ops::RangeToInclusive<T: Clone + Send + Sync>(Clone));
impl_reflect_opaque!(::core::ops::RangeFull(Clone));
impl_reflect_opaque!(::core::ops::Bound<T: Clone + Send + Sync>(Clone));
