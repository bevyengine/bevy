/// TODO: Docs
#[macro_export]
macro_rules! bind_group_descriptor {
    (@entry, texture, $binding_index: expr, $resource_value: expr) => {
        $crate::render_resource::BindGroupEntry {
            binding: $binding_index,
            resource: $crate::render_resource::BindingResource::TextureView($resource_value),
        }
    };
    (@entry, sampler, $binding_index: expr, $resource_value: expr) => {
        $crate::render_resource::BindGroupEntry {
            binding: $binding_index,
            resource: $crate::render_resource::BindingResource::Sampler($resource_value),
        }
    };
    (@entry, buffer, $binding_index: expr, $resource_value: expr) => {
        $crate::render_resource::BindGroupEntry {
            binding: $binding_index,
            resource: $resource_value,
        }
    };
    (@entry, $any_other:ident, $binding_index: expr, $resource_value: expr) => {
        compile_error!("only 'texture', 'sampler', and 'buffer' are supported")
    };
    ($label:expr, $layout:expr, $($entry_kind: ident ($resource_value:expr)),*,) => {{
        let mut i = 0;
        &$crate::render_resource::BindGroupDescriptor {
            label: Some($label),
            layout: $layout,
            entries: &[ $(
                bind_group_descriptor!(@entry, $entry_kind, { i += 1; i - 1}, $resource_value),
            )* ],
        }
    }};
}
