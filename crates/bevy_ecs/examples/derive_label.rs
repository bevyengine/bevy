use std::marker::PhantomData;

use bevy_ecs::prelude::*;

fn main() {
    // Unit labels are always equal.
    assert_eq!(UnitLabel.as_label(), UnitLabel.as_label());

    // Enum labels depend on the variant.
    assert_eq!(EnumLabel::One.as_label(), EnumLabel::One.as_label());
    assert_ne!(EnumLabel::One.as_label(), EnumLabel::Two.as_label());

    // Labels annotated with `ignore_fields` ignore their fields.
    assert_eq!(WeirdLabel(1).as_label(), WeirdLabel(2).as_label());

    // Labels don't depend only on the variant name but on the full type
    assert_ne!(
        GenericLabel::<f64>::One.as_label(),
        GenericLabel::<char>::One.as_label(),
    );
}

#[derive(SystemLabel)]
pub struct UnitLabel;

#[derive(SystemLabel)]
pub enum EnumLabel {
    One,
    Two,
}

#[derive(SystemLabel)]
#[system_label(ignore_fields)]
pub struct WeirdLabel(i32);

#[derive(SystemLabel)]
pub enum GenericLabel<T> {
    One,
    #[system_label(ignore_fields)]
    Two(PhantomData<T>),
}

// FIXME: this should be a compile_fail test
/*#[derive(SystemLabel)]
pub union Foo {
    x: i32,
}*/

// FIXME: this should be a compile_fail test
/*#[derive(SystemLabel)]
#[system_label(ignore_fields)]
pub enum BadLabel {
    One,
    Two,
}*/

// FIXME: this should be a compile_fail test
/*#[derive(SystemLabel)]
pub struct BadLabel2 {
    #[system_label(ignore_fields)]
    x: (),
}*/
