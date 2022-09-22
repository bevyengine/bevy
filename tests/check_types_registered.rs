use std::any::TypeId;

use bevy::{
    prelude::*,
    reflect::{TypeInfo, TypeRegistryInternal, VariantInfo},
};

#[test]
fn check_types_registered_recursive() {
    let mut app = App::new();
    app.add_plugins_with(DefaultPlugins, |g| g.disable::<bevy::winit::WinitPlugin>());

    let type_registry = app.world.resource::<AppTypeRegistry>();
    let type_registry = type_registry.read();

    for registration in type_registry.iter() {
        assert_registered_recursive(
            &*type_registry,
            registration.type_id(),
            registration.type_name(),
        );
    }
}

fn assert_registered_recursive(
    type_registry: &TypeRegistryInternal,
    type_id: TypeId,
    name: &'static str,
) {
    let registration = type_registry
        .get(type_id)
        .unwrap_or_else(|| panic!("{name} is not registered"));
    match registration.type_info() {
        TypeInfo::Struct(info) => info.iter().for_each(|field| {
            assert_registered_recursive(type_registry, field.type_id(), field.type_name())
        }),
        TypeInfo::TupleStruct(info) => info.iter().for_each(|field| {
            assert_registered_recursive(type_registry, field.type_id(), field.type_name())
        }),
        TypeInfo::Tuple(info) => info.iter().for_each(|field| {
            assert_registered_recursive(type_registry, field.type_id(), field.type_name())
        }),
        TypeInfo::List(info) => {
            assert_registered_recursive(type_registry, info.item_type_id(), info.item_type_name())
        }
        TypeInfo::Array(info) => {
            assert_registered_recursive(type_registry, info.item_type_id(), info.item_type_name())
        }
        TypeInfo::Map(info) => {
            assert_registered_recursive(type_registry, info.key_type_id(), info.key_type_name());
            assert_registered_recursive(
                type_registry,
                info.value_type_id(),
                info.value_type_name(),
            );
        }
        TypeInfo::Enum(info) => info.iter().for_each(|variant| match variant {
            VariantInfo::Struct(variant) => variant.iter().for_each(|field| {
                assert_registered_recursive(type_registry, field.type_id(), field.type_name())
            }),
            VariantInfo::Tuple(variant) => variant.iter().for_each(|field| {
                assert_registered_recursive(type_registry, field.type_id(), field.type_name())
            }),
            VariantInfo::Unit(_) => {}
        }),
        TypeInfo::Value(_) => {}
        TypeInfo::Dynamic(_) => todo!(),
    }
}
