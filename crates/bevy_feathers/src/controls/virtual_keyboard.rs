use bevy_ecs::prelude::*;
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_scene2::prelude::*;
use bevy_ui::{widget::Text, FlexDirection, Node, Val};
use bevy_ui_widgets::Activate;

use crate::controls::{button, ButtonProps};

/// Fired whenever a virtual key is pressed.
#[derive(EntityEvent)]
pub struct VirtualKeyPressed<T> {
    /// The virtual keyboard entity
    pub entity: Entity,
    /// The pressed virtual key
    pub key: T,
}

/// Function to spawn a virtual keyboard
pub fn virtual_keyboard<T>(keys: impl Iterator<Item = Vec<T>> + Send + Sync + 'static) -> impl Scene
where
    T: AsRef<str> + Clone + Send + Sync + 'static,
{
    let keys = Vec::from_iter(keys.map(move |row| {
        let key_row = Vec::from_iter(row.into_iter().map(move |key| {
            let key_clone = key.clone();
            bsn! {
                button(ButtonProps::default())
                on(
                    move |activate: On<Activate>,
                          mut commands: Commands,
                          query: Query<&ChildOf>|
                          -> Result {
                        let virtual_keyboard =
                            query.get(query.get(activate.entity)?.parent())?.parent();
                        commands.trigger(VirtualKeyPressed {
                            entity: virtual_keyboard,
                            key: key.clone(),
                        });
                        Ok(())
                    },
                )
                [
                    Text::new(key_clone.as_ref())
                ]
            }
        }));
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.),
            }
            [
              {key_row}
            ]
        }
    }));
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.),
        }
        TabGroup::new(0)
        [
            {keys}
        ]
    }
}
