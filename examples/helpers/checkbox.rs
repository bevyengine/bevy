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

impl IsChecked {
    fn checked(&self) -> bool {
        match self {
            IsChecked(true) => true,
            IsChecked(false) => false,
        }
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
        {
            checkbox_identifier.map(|checkbox_identifier| {
            bsn! { template_value(checkbox_identifier) }
        })}
        on(checkbox_self_update)
        {status.checked().then(|| bsn! { Checked })}
        ]
    })
}
