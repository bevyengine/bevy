//! Demonstrates how to annotate reflected types with well-known annotations and read them back,
//! the way an inspector or editor would.

use bevy::{
    annotations::{DisplayName, ReadOnly, Step, Tooltip, Unit, ValueRange},
    prelude::*,
    reflect::{TypeInfo, TypeRegistration, TypeRegistry, Typed},
};

#[derive(Component, Reflect)]
#[reflect(Component, @Tooltip("Movement settings for a character"))]
struct Movement {
    #[reflect(@ValueRange(0.0..=20.0), @Step(0.5), @Unit("m/s"))]
    speed: f32,
    #[reflect(@DisplayName("Jump Height"), @Tooltip("Peak height of a jump"))]
    jump: f32,
    #[reflect(@ReadOnly)]
    grounded: bool,
    mode: Mode,
}

#[derive(Reflect)]
enum Mode {
    #[reflect(@DisplayName("On Foot"))]
    Walking,
    #[reflect(@Tooltip("Faster, but uses stamina"))]
    Running,
}

fn main() {
    let mut registry = TypeRegistry::default();
    registry.register::<Movement>();
    registry.register::<Mode>();

    describe::<Movement>(&registry);
}

fn describe<T: Typed>(registry: &TypeRegistry) {
    let registration = registry.get(std::any::TypeId::of::<T>()).unwrap();
    let TypeInfo::Struct(info) = registration.type_info() else {
        panic!("expected struct");
    };

    println!("{}", info.type_path_table().short_path());
    if let Some(tooltip) = info.get_attribute::<Tooltip>() {
        println!("  {}", tooltip.0);
    }

    for field in info.iter() {
        let name = field
            .get_attribute::<DisplayName>()
            .map_or(field.name(), |display| display.0);
        println!("- {name}");

        if let Some(range) = ValueRange::from_attributes(field.custom_attributes()) {
            println!("    range: {}..={}", range.min(), range.max());
        }
        if let Some(step) = field.get_attribute::<Step>() {
            println!("    step: {}", step.0);
        }
        if let Some(unit) = field.get_attribute::<Unit>() {
            println!("    unit: {}", unit.0);
        }
        if let Some(tooltip) = field.get_attribute::<Tooltip>() {
            println!("    tooltip: {}", tooltip.0);
        }
        if field.has_attribute::<ReadOnly>() {
            println!("    read only");
        }

        let Some(TypeInfo::Enum(variants)) = registry
            .get(field.type_id())
            .map(TypeRegistration::type_info)
        else {
            continue;
        };
        for variant in variants.iter() {
            let name = variant
                .get_attribute::<DisplayName>()
                .map_or(variant.name(), |display| display.0);
            match variant.get_attribute::<Tooltip>() {
                Some(tooltip) => println!("    variant: {name} ({})", tooltip.0),
                None => println!("    variant: {name}"),
            }
        }
    }
}
