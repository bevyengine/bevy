use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::{Changed, FromWorld, Query, Res, With, Without, World};
use bevy_render::prelude::Color;
use bevy_sprite::ColorMaterial;

use crate::Interaction;

#[derive(Debug, Clone)]
pub struct Button;

#[derive(Debug, Clone)]
pub struct DefaultButtonMaterials {
    pub normal: Handle<ColorMaterial>,
    pub hovered: Handle<ColorMaterial>,
    pub pressed: Handle<ColorMaterial>,
}

impl FromWorld for DefaultButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();

        DefaultButtonMaterials {
            normal: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
            hovered: materials.add(Color::rgb(0.25, 0.25, 0.25).into()),
            pressed: materials.add(Color::rgb(0.35, 0.75, 0.35).into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonMaterials {
    pub normal: Handle<ColorMaterial>,
    pub hovered: Handle<ColorMaterial>,
    pub pressed: Handle<ColorMaterial>,
}

#[derive(Debug, Clone)]
pub struct CustomButtonMaterialBehavior;

pub fn button_default_materials_system(
    default_materials: Res<DefaultButtonMaterials>,
    mut query: Query<
        (&Interaction, &mut Handle<ColorMaterial>),
        (
            Changed<Interaction>,
            With<Button>,
            Without<ButtonMaterials>,
            Without<CustomButtonMaterialBehavior>,
        ),
    >,
) {
    for (interaction, mut material) in query.iter_mut() {
        match *interaction {
            Interaction::None => {
                *material = default_materials.normal.clone();
            }
            Interaction::Hovered => {
                *material = default_materials.hovered.clone();
            }
            Interaction::Clicked => {
                *material = default_materials.pressed.clone();
            }
        }
    }
}

pub fn button_materials_system(
    mut query: Query<
        (&Interaction, &mut Handle<ColorMaterial>, &ButtonMaterials),
        (
            Changed<Interaction>,
            With<Button>,
            Without<CustomButtonMaterialBehavior>,
        ),
    >,
) {
    for (interaction, mut material, materials) in query.iter_mut() {
        match *interaction {
            Interaction::None => {
                *material = materials.normal.clone();
            }
            Interaction::Hovered => {
                *material = materials.hovered.clone();
            }
            Interaction::Clicked => {
                *material = materials.pressed.clone();
            }
        }
    }
}
