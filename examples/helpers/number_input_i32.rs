/// Helpers to create a basic i32 number input using `FeathersNumberInput`
/// Using these helpers requires the `bevy_feathers` feature to be enabled.
use bevy::{
    feathers::{
        controls::{FeathersNumberInput, HardLimit, NumberInputPrecision, NumberInputValue},
        display::label,
    },
    prelude::*,
};

/// Creates an i32 number input.
///
/// `number_input_identifier` should be a component that will distinguish this
/// number input from any others if needed.
///
/// Examples that use this to create a number input should handle its `ValueChange<i32>` events.
/// If there is a need to identify the number input that originated the value change,
/// query which `number_input_identifier` with the `FeathersNumberInput` is
/// the value change's source entity.
pub fn number_input_i32<T>(
    name: &'static str,
    number_input_identifier: Option<T>,
    value: i32,
    precision: NumberInputPrecision,
    limits: core::ops::Range<i32>,
) -> Box<dyn Scene>
where
    T: Template<Output: Component> + Send + Sync + Unpin + 'static,
{
    if let Some(identifier) = number_input_identifier {
        Box::new(bsn! {
            Node {
                align_items: AlignItems::Center,
                column_gap: px(5),
            }
            Children [
                Node {
                    align_items: AlignItems::Center,
                    width: px(150),
                }
                Children [
                    label(name)
                ],

                Node {
                    align_items: AlignItems::Center,
                }
                template_value(identifier)
                @FeathersNumberInput
                template_value(NumberInputValue::I32(value))
                template_value(precision)
                HardLimit::i32(limits)
            ]
        })
    } else {
        Box::new(bsn! {
            Node {
                align_items: AlignItems::Center,
                column_gap: px(5),
            }
            Children [
                Node {
                    align_items: AlignItems::Center,
                    width: px(150),
                }
                Children [
                    label(name)
                ],

                Node {
                    align_items: AlignItems::Center,
                }
                @FeathersNumberInput
                template_value(NumberInputValue::I32(value))
                template_value(precision)
                HardLimit::i32(limits)
            ]
        })
    }
}
