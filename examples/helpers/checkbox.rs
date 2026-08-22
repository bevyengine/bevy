/// Helpers to create an option menu using Feathers Checkboxes.
/// Using these helpers requires the `bevy_feathers` feature to be enabled.
use bevy::{
    feathers::{controls::FeathersCheckbox, display::caption},
    picking::hover::Hovered,
    prelude::*,
    ui::Checked,
    ui_widgets::checkbox_self_update,
};

/// A newtype bool wrapper to indicate a widget's checked status.
pub struct IsChecked(pub bool);

/// Helper to create a single checked feathers checkbox.
fn checked_checkbox<T>(option_name: &str, checkbox_identifier: Option<T>) -> Box<dyn Scene>
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
                    @caption: bsn! { caption(option_name) }
                }
                Checked
                Hovered::default()
                template_value(identifier)
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
                    @caption: bsn! { caption(option_name) }
                }
                Checked
                Hovered::default()
                on(checkbox_self_update)
            ]
        })
    }
}

/// Helper to create a single unchecked feathers checkbox.
fn unchecked_checkbox<T>(option_name: &str, checkbox_identifier: Option<T>) -> Box<dyn Scene>
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
                    @caption: bsn! { caption(option_name) }
                }
                Hovered::default()
                template_value(identifier)
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
                    @caption: bsn! { caption(option_name) }
                }
                Hovered::default()
                on(checkbox_self_update)
            ]
        })
    }
}

/// Creates a single feathers checkbox that allows configuration of a setting.  
///
/// Examples that use this to create a checkbox should handle its `ValueChange<bool>` events.
/// If there is a need to identify the checkbox that originated the value change,
/// query which `checkbox_identifier` with the `FeathersCheckbox` is the value change's
/// source entity.
pub fn feathers_option_checkbox<T>(
    option_name: &str,
    checkbox_identifier: Option<T>,
    status: IsChecked,
) -> Box<dyn Scene>
where
    T: Template<Output: Component> + Clone + Default + Send + Sync + Unpin + 'static,
{
    match status {
        IsChecked(true) => checked_checkbox(option_name, checkbox_identifier),
        IsChecked(false) => unchecked_checkbox(option_name, checkbox_identifier),
    }
}
