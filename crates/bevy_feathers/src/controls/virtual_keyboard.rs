use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    hierarchy::{ChildOf, Children},
    relationship::RelatedSpawner,
    spawn::{Spawn, SpawnRelated, SpawnWith},
    system::{In, SystemId},
};
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_ui::Node;
use bevy_ui::Val;
use bevy_ui::{widget::Text, FlexDirection};
use bevy_ui_widgets::{Activate, Callback};

use crate::controls::{button, ButtonProps};

/// Function to spawn a virtual keyboard
pub fn virtual_keyboard<T>(
    keys: impl Iterator<Item = Vec<(String, T)>> + Send + Sync + 'static,
    on_key_press: SystemId<In<Activate>>,
) -> impl Bundle
where
    T: Component,
{
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.),
            ..Default::default()
        },
        TabGroup::new(0),
        Children::spawn((SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            for row in keys {
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.),
                        ..Default::default()
                    },
                    Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
                        for (label, key_id) in row.into_iter() {
                            parent.spawn(button(
                                ButtonProps {
                                    on_click: Callback::System(on_key_press),
                                    ..Default::default()
                                },
                                (key_id,),
                                Spawn(Text::new(label)),
                            ));
                        }
                    })),
                ));
            }
        }),)),
    )
}
