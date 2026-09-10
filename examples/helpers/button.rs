/// Helpers to create an action button using `FeathersButtons`.
/// Using these helpers requires the `bevy_feathers` feature to be enabled.
use bevy::{
    feathers::{controls::FeathersButton, display::caption},
    picking::hover::Hovered,
    prelude::*,
};

/// Creates a single feathers button that allows activation of a setting.  
///
/// Examples that use this to create a button should handle its `On<Activate>` trigger.
/// If there is a need to identify the button that originated the activation trigger,
/// query which `button_identifier` with the `FeathersButton` is the trigger's source entity.
pub fn feathers_button<T>(option_name: &str, button_identifier: Option<T>) -> Box<dyn Scene>
where
    T: Template<Output: Component> + Clone + Default + Send + Sync + Unpin + 'static,
{
    if let Some(identifier) = button_identifier {
        Box::new(bsn! {
            Node {
                align_items: AlignItems::Center,
                column_gap: px(5),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { caption(option_name) }
                }
                Hovered::default()
                template_value(identifier)
            ]
        })
    } else {
        Box::new(bsn! {
            Node {
                align_items: AlignItems::Center,
                column_gap: px(5),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { caption(option_name) }
                }
                Hovered::default()
            ]
        })
    }
}
