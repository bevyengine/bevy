use zerocopy::{FromBytes, AsBytes};
use crate::{Vec2, Vec3, Vec4, Mat2, Mat3, Mat4, Quat};

unsafe impl AsBytes for Vec2 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl FromBytes for Vec2 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl AsBytes for Vec3 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl FromBytes for Vec3 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl AsBytes for Vec4 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl FromBytes for Vec4 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl AsBytes for Mat2 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl FromBytes for Mat2 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl AsBytes for Mat3 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl FromBytes for Mat3 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl AsBytes for Mat4 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl FromBytes for Mat4 {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl AsBytes for Quat {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}

unsafe impl FromBytes for Quat {
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized {}
    
}