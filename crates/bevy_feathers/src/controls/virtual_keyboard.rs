use bevy_ecs::prelude::*;
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_scene::prelude::*;
use bevy_ui::Node;
use bevy_ui::Val;
use bevy_ui::{widget::Text, FlexDirection};
use bevy_ui_widgets::{observe, Activate};

use crate::controls::button::{button, ButtonBundleProps, ButtonProps};
use crate::controls::button_bundle;

/// Fired whenever a virtual key is pressed.
#[derive(EntityEvent)]
pub struct VirtualKeyPressed<T> {
    /// The virtual keyboard entity
    pub entity: Entity,
    /// The pressed virtual key
    pub key: T,
}

/// Function to spawn a virtual keyboard
///
/// # Emitted events
/// * [`crate::controls::VirtualKeyPressed<T>`] when a virtual key on the keyboard is un-pressed.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
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
                Children [
                    Text::new(key_clone.as_ref())
                ]
            }
        }));
        bsn! {
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.),
            }
            Children [
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
        Children [
            {keys}
        ]
    }
}

/// Function to spawn a virtual keyboard
///
/// # Emitted events
/// * [`crate::controls::VirtualKeyPressed<T>`] when a virtual key on the keyboard is un-pressed.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
#[deprecated(since = "0.19.0", note = "Use the virtual_keyboard() BSN function")]
#[expect(deprecated, reason = "uses the deprecated button_bundle")]
pub fn virtual_keyboard_bundle<T>(
    keys: impl Iterator<Item = Vec<T>> + Send + Sync + 'static,
) -> impl Bundle
where
    T: AsRef<str> + Clone + Send + Sync + 'static,
{
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.),
            ..Default::default()
        },
        TabGroup::new(0),
        Children::spawn(SpawnIter(keys.map(move |row| {
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.),
                    ..Default::default()
                },
                Children::spawn(SpawnIter(row.into_iter().map(move |key| {
                    (
                        button_bundle(
                            ButtonBundleProps::default(),
                            (),
                            Spawn(Text::new(key.as_ref())),
                        ),
                        observe(
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
                        ),
                    )
                }))),
            )
        }))),
    )
}
