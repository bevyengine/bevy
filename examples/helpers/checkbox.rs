/// Helpers to create an option menu using Feathers Checkboxes.
/// Using these helpers requires the `bevy_feathers` feature to be enabled.
use bevy::{
    feathers::{controls::FeathersCheckbox, display::caption},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::checkbox_self_update,
};

/// Creates a single feathers checkbox that allows configuration of a setting.
///
/// Examples that use this to create a checkbox should handle its `ValueChange<bool>` events.
/// If there is a need to identify the checkbox that originated the value change,
/// query which `checkbox_identifier` with the `FeathersCheckbox` is the value change's
/// source entity.
pub fn feathers_option_checkbox<T>(
    option_name: &str,
    checkbox_identifier: Option<T>,
) -> Box<dyn Scene>
where
    T: Template<Output: Component> + Clone + Default + Send + Sync + Unpin + 'static,
{
    if let Some(identifier) = checkbox_identifier {
        Box::new(bsn! {
            Node {
                align_items: AlignItems::Center,
                column_gap: px(5),
            }
            Children [
                @FeathersCheckbox {
                    @caption: bsn! { @caption(option_name) }
                }
                Hovered
                identifier
                on(checkbox_self_update)
            ]
        })
    } else {
        Box::new(bsn! {
            Node {
                align_items: AlignItems::Center,
                column_gap: px(5),
            }
            Children [
                @FeathersCheckbox {
                    @caption: bsn! { @caption(option_name) }
                }
                Hovered
                on(checkbox_self_update)
            ]
        })
    }
}
