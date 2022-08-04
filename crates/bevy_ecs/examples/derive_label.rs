use std::{
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
};

use bevy_ecs::{
    prelude::*,
    schedule::{LabelGuard, Labels, SystemLabelId},
};

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

    // Working with labels that need to be heap allocated.
    let label = ComplexLabel {
        people: vec!["John", "William", "Sharon"],
    };
    // Convert to to a LabelId. Its type gets erased.
    let id = label.as_label();
    assert_eq!(
        format!("{id:?}"),
        r#"ComplexLabel { people: ["John", "William", "Sharon"] }"#
    );
    // Try to downcast it back to its concrete type.
    if let Some(complex_label) = id.downcast::<ComplexLabel>() {
        assert_eq!(complex_label.people, vec!["John", "William", "Sharon"]);
    } else {
        // The downcast will never fail in this example, since the label is always
        // created from a value of type `ComplexLabel`.
        unreachable!();
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComplexLabel {
    people: Vec<&'static str>,
}

static MAP: Labels<ComplexLabel> = Labels::new();

fn compute_hash(val: &impl Hash) -> u64 {
    let mut hasher = bevy_utils::FixedState.build_hasher();
    val.hash(&mut hasher);
    hasher.finish()
}

impl SystemLabel for ComplexLabel {
    fn data(&self) -> u64 {
        let hash = compute_hash(self);
        MAP.intern(hash, || self.clone());
        hash
    }
    fn fmt(hash: u64, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        MAP.scope(hash, |val| write!(f, "{val:?}"))
            .ok_or_else(Default::default)?
    }
}

impl bevy_utils::label::LabelDowncast<SystemLabelId> for ComplexLabel {
    type Output = LabelGuard<'static, ComplexLabel>;
    fn downcast_from(label: SystemLabelId) -> Option<Self::Output> {
        let hash = label.data();
        MAP.get(hash)
    }
}
