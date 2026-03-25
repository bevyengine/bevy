use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::bevy_platform::time::Instant(
    Clone, Debug, Hash, PartialEq
));

#[cfg(test)]
mod tests {
    use crate::FromReflect;
    use bevy_platform::time::Instant;

    #[test]
    fn instant_should_from_reflect() {
        let expected = Instant::now();
        let output = Instant::from_reflect(&expected).unwrap();
        assert_eq!(expected, output);
    }
}
