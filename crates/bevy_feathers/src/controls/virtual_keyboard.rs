use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    hierarchy::Children,
    observer::On,
    spawn::{Spawn, SpawnIter, SpawnRelated},
    system::{Commands, In, SystemId},
};
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_ui::Node;
use bevy_ui::Val;
use bevy_ui::{widget::Text, FlexDirection};
use bevy_ui_widgets::{observe, Activate};

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
        Children::spawn(SpawnIter(keys.map(move |row| {
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.),
                    ..Default::default()
                },
                Children::spawn(SpawnIter(row.into_iter().map(move |(label, key_id)| {
                    (
                        button(ButtonProps::default(), (key_id,), Spawn(Text::new(label))),
                        observe(move |activate: On<Activate>, mut commands: Commands| {
                            // TODO: Turn this into an event as well, or use event forwarding.
                            commands.run_system_with(on_key_press, *activate);
                        }),
                    )
                }))),
            )
        }))),
    )
}
