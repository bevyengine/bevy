use std::{fmt::Debug, hash::Hash, marker::PhantomData};

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

    assert_eq!(format!("{:?}", UnitLabel.as_label()), "UnitLabel");
    assert_eq!(format!("{:?}", WeirdLabel(1).as_label()), "WeirdLabel");
    assert_eq!(format!("{:?}", WeirdLabel(2).as_label()), "WeirdLabel");
    assert_eq!(
        format!("{:?}", GenericLabel::<f64>::One.as_label()),
        "GenericLabel::One::<f64>"
    );
    assert_eq!(
        format!("{:?}", ConstGenericLabel::<21>.as_label()),
        "ConstGenericLabel::<21>"
    );

    // Working with labels that need to be heap allocated.
    let label = ComplexLabel {
        people: vec!["John", "William", "Sharon"],
    };
    // Convert it to a LabelId. Its type gets erased.
    let id = label.as_label();
    assert_eq!(
        format!("{id:?}"),
        r#"ComplexLabel { people: ["John", "William", "Sharon"] }"#
    );

    // Generic heap-allocated labels.
    let id = WrapLabel(1_i128).as_label();
    assert_eq!(format!("{id:?}"), "WrapLabel(1)");

    // Different types with the same type constructor.
    let id2 = WrapLabel(1_u32).as_label();
    // The debug representations are the same...
    assert_eq!(format!("{id:?}"), format!("{id2:?}"));
    // ...but they do not compare equal.
    assert_ne!(id, id2);
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

#[derive(SystemLabel)]
pub struct ConstGenericLabel<const N: usize>;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemLabel)]
#[system_label(intern)]
pub struct ComplexLabel {
    people: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemLabel)]
#[system_label(intern)]
pub struct WrapLabel<T>(T);
