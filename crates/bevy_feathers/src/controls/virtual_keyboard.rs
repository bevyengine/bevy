use bevy_core_widgets::{Activate, CallbackTemplate};
use bevy_ecs::{
    component::Component,
    system::{In, SystemId},
    template::template,
};
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_scene2::prelude::*;
use bevy_ui::Node;
use bevy_ui::Val;
use bevy_ui::{widget::Text, FlexDirection};

use crate::controls::{button, ButtonProps};

/// Function to spawn a virtual keyboard
pub fn virtual_keyboard<T: Component + Clone>(
    keys: impl Iterator<Item = Vec<(String, T)>> + Send + Sync + 'static,
    on_key_press: SystemId<In<Activate>>,
) -> impl Scene {
    let children: Vec<_> = keys.map(|row| {
        let children: Vec<_> = row.into_iter().map(|(label, key_id)| {
            bsn! {
                :button(ButtonProps { on_click: CallbackTemplate::SystemId(on_key_press), ..Default::default() })
                template(move |entity| {entity.insert(key_id.clone()); Ok(())})
                [Text::new(label.clone())]
            }
        }).collect();

        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.),
            }
            [{ children }]
        }
    }).collect();
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.),
        }
        TabGroup::new(0)
        [ {children} ]
    }
}
